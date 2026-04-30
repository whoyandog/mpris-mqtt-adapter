use anyhow::{Context, Result};

use crate::playerctl::run_playerctl;
use crate::types::CmdMsg;

pub fn parse_command(payload: &str) -> Result<CmdMsg> {
    let msg: CmdMsg = serde_json::from_str(payload).context("command payload must be valid JSON")?;
    Ok(msg)
}

pub fn handle_command(player: &str, payload: &str) -> Result<()> {
    let cmd = parse_command(payload)?;

    match cmd.action.as_str() {
        "play" => {
            run_playerctl(player, &["play"])?;
        }
        "pause" => {
            run_playerctl(player, &["pause"])?;
        }
        "play_pause" | "toggle" => {
            run_playerctl(player, &["play-pause"])?;
        }
        "next" => {
            run_playerctl(player, &["next"])?;
        }
        "prev" | "previous" => {
            run_playerctl(player, &["previous"])?;
        }
        "stop" => {
            run_playerctl(player, &["stop"])?;
        }
        "volume_set" => {
            if let Some(value) = cmd.value.and_then(|v| v.as_f64()) {
                run_playerctl(player, &["volume", &value.to_string()])?;
            } else {
                anyhow::bail!("volume_set requires numeric value")
            }
        }
        "volume_up" => {
            if let Some(value) = cmd.value.and_then(|v| v.as_f64()) {
                run_playerctl(player, &["volume", &format!("+{}", value)])?;
            } else {
                run_playerctl(player, &["volume", "+0.05"])?;
            }
        }
        "volume_down" => {
            if let Some(value) = cmd.value.and_then(|v| v.as_f64()) {
                run_playerctl(player, &["volume", &format!("-{}", value)])?;
            } else {
                run_playerctl(player, &["volume", "-0.05"])?;
            }
        }
        "mute" => {
            run_playerctl(player, &["volume", "0"])?;
        }
        "position_set" => {
            if let Some(value) = cmd.value.and_then(|v| v.as_f64()) {
                run_playerctl(player, &["position", &value.to_string()])?;
            } else {
                anyhow::bail!("position_set requires numeric value in seconds")
            }
        }
        "position_seek" => {
            if let Some(value) = cmd.value.and_then(|v| v.as_f64()) {
                if value >= 0.0 {
                    run_playerctl(player, &["position", &format!("+{}", value)])?;
                } else {
                    run_playerctl(player, &["position", &value.to_string()])?;
                }
            } else {
                anyhow::bail!("position_seek requires numeric value in seconds")
            }
        }
        "loop_none" => {
            run_playerctl(player, &["loop", "None"])?;
        }
        "loop_track" => {
            run_playerctl(player, &["loop", "Track"])?;
        }
        "loop_playlist" => {
            run_playerctl(player, &["loop", "Playlist"])?;
        }
        "shuffle_on" => {
            run_playerctl(player, &["shuffle", "On"])?;
        }
        "shuffle_off" => {
            run_playerctl(player, &["shuffle", "Off"])?;
        }
        _ => anyhow::bail!("unknown action: {}", cmd.action),
    }

    Ok(())
}
