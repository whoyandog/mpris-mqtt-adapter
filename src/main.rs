use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use clap::Parser;
use rumqttc::{AsyncClient, Event, Incoming, LastWill, MqttOptions, QoS};
mod commands;
mod config;
mod discovery;
mod playerctl;
mod types;
mod util;

use commands::handle_command;
use config::Cli;
use discovery::publish_discovery;
use playerctl::{detect_capabilities_with_probe, read_state};
use types::{Capabilities, PlayerState};
use util::sanitize;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut opts = MqttOptions::new("mpris-mqtt-adapter", cli.host.clone(), cli.port);
    opts.set_keep_alive(Duration::from_secs(10));

    let availability_topic = format!("{}/availability", cli.topic);
    opts.set_last_will(LastWill::new(
        availability_topic.clone(),
        "offline",
        QoS::AtLeastOnce,
        true,
    ));

    if let (Ok(username), Ok(password)) = (
        std::env::var("MQTT_USERNAME"),
        std::env::var("MQTT_PASSWORD"),
    ) {
        opts.set_credentials(username, password);
    }

    let cmd_topic = format!("{}/cmd", cli.topic);
    let state_topic = format!("{}/state", cli.topic);
    let capabilities_topic = format!("{}/capabilities", cli.topic);
    let event_topic = format!("{}/event", cli.topic);

    let (client, mut eventloop) = AsyncClient::new(opts, 50);

    client
        .publish(
            availability_topic.clone(),
            QoS::AtLeastOnce,
            true,
            "online",
        )
        .await
        .context("failed to publish availability online status")?;

    client
        .subscribe(cmd_topic.clone(), QoS::AtLeastOnce)
        .await
        .context("failed to subscribe to command topic")?;

    let (capabilities, probe_report) = detect_capabilities_with_probe(&cli.player);
    client
        .publish(
            capabilities_topic.clone(),
            QoS::AtLeastOnce,
            true,
            serde_json::to_vec(&capabilities)?,
        )
        .await
        .context("failed to publish capabilities")?;

    if cli.probe_diagnostics {
        client
            .publish(
                event_topic.clone(),
                QoS::AtLeastOnce,
                false,
                serde_json::to_vec(&probe_report)?,
            )
            .await
            .context("failed to publish initial probe diagnostics")?;
    }

    if cli.discovery {
        publish_discovery(&client, &cli.topic, &state_topic, &cmd_topic).await?;
    }

    let mut ticker = tokio::time::interval(Duration::from_secs(cli.poll_seconds));
    let mut last_state: Option<PlayerState> = None;
    let mut last_capabilities: Option<Capabilities> = Some(capabilities);
    let mut last_probe_payload: Option<Vec<u8>> = if cli.probe_diagnostics {
        Some(serde_json::to_vec(&probe_report)?)
    } else {
        None
    };
    let mut last_active_player: Option<String> = probe_report.resolved_player.clone();
    let capabilities_ttl = Duration::from_secs(cli.capabilities_ttl_seconds.max(1));
    let mut last_capabilities_refresh = Instant::now();

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let mut player_changed = false;

                if let Ok(state) = read_state(&cli.player) {
                    if last_active_player.as_ref() != Some(&state.player) {
                        last_active_player = Some(state.player.clone());
                        player_changed = true;
                    }

                    if last_state.as_ref() != Some(&state) {
                        let payload = serde_json::to_vec(&state)?;
                        client.publish(state_topic.clone(), QoS::AtLeastOnce, true, payload).await?;
                        last_state = Some(state);
                    }
                }

                let ttl_expired = last_capabilities_refresh.elapsed() >= capabilities_ttl;
                if ttl_expired || player_changed || last_capabilities.is_none() {
                    let (capabilities, probe_report) = detect_capabilities_with_probe(&cli.player);

                    if last_capabilities.as_ref() != Some(&capabilities) {
                        let payload = serde_json::to_vec(&capabilities)?;
                        client.publish(capabilities_topic.clone(), QoS::AtLeastOnce, true, payload).await?;
                        last_capabilities = Some(capabilities);
                    }

                    if cli.probe_diagnostics {
                        let payload = serde_json::to_vec(&probe_report)?;
                        if last_probe_payload.as_ref() != Some(&payload) {
                            client.publish(event_topic.clone(), QoS::AtLeastOnce, false, payload.clone()).await?;
                            last_probe_payload = Some(payload);
                        }
                    }

                    last_capabilities_refresh = Instant::now();
                }
            }
            event = eventloop.poll() => {
                match event {
                    Ok(Event::Incoming(Incoming::Publish(publish))) => {
                        if publish.topic == cmd_topic {
                            let payload = String::from_utf8_lossy(&publish.payload).to_string();
                            if let Err(err) = handle_command(&cli.player, &payload) {
                                let msg = format!(
                                    "{{\"status\":\"error\",\"message\":\"{}\"}}",
                                    sanitize(&err.to_string())
                                );
                                let _ = client.publish(event_topic.clone(), QoS::AtLeastOnce, false, msg).await;
                            } else if let Ok(state) = read_state(&cli.player) {
                                let payload = serde_json::to_vec(&state)?;
                                client
                                    .publish(state_topic.clone(), QoS::AtLeastOnce, true, payload)
                                    .await?;

                                last_active_player = Some(state.player.clone());
                                last_state = Some(state);
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(err) => {
                        let msg = format!(
                            "{{\"status\":\"mqtt-error\",\"message\":\"{}\"}}",
                            sanitize(&err.to_string())
                        );
                        let _ = client.publish(event_topic.clone(), QoS::AtLeastOnce, false, msg).await;
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }
    }
}

