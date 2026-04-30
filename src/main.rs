use std::time::Duration;

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
use playerctl::{detect_capabilities, read_state};
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

    let capabilities = detect_capabilities(&cli.player).unwrap_or_else(|_| Capabilities::unavailable());
    client
        .publish(
            capabilities_topic.clone(),
            QoS::AtLeastOnce,
            true,
            serde_json::to_vec(&capabilities)?,
        )
        .await
        .context("failed to publish capabilities")?;

    if cli.discovery {
        publish_discovery(&client, &cli.topic, &state_topic, &cmd_topic).await?;
    }

    let mut ticker = tokio::time::interval(Duration::from_secs(cli.poll_seconds));
    let mut last_state: Option<PlayerState> = None;
    let mut last_capabilities: Option<Capabilities> = Some(capabilities);

    loop {
        tokio::select! {
            _ = ticker.tick() => {
                let capabilities = detect_capabilities(&cli.player).unwrap_or_else(|_| Capabilities::unavailable());
                if last_capabilities.as_ref() != Some(&capabilities) {
                    let payload = serde_json::to_vec(&capabilities)?;
                    client.publish(capabilities_topic.clone(), QoS::AtLeastOnce, true, payload).await?;
                    last_capabilities = Some(capabilities);
                }

                if let Ok(state) = read_state(&cli.player) {
                    if last_state.as_ref() != Some(&state) {
                        let payload = serde_json::to_vec(&state)?;
                        client.publish(state_topic.clone(), QoS::AtLeastOnce, true, payload).await?;
                        last_state = Some(state);
                    }
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

