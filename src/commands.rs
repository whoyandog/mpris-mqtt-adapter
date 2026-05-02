use anyhow::{Context, Result};
use serde_json::Value;

use crate::playerctl::run_playerctl;
use crate::types::CmdMsg;

pub fn parse_command(payload: &str) -> Result<CmdMsg> {
    let msg: CmdMsg = serde_json::from_str(payload).context("command payload must be valid JSON")?;
    Ok(msg)
}

fn parse_numeric_value(value: Option<&Value>, required: bool, field_name: &str) -> Result<Option<f64>> {
    match value.and_then(Value::as_f64) {
        Some(num) => Ok(Some(num)),
        None if required => anyhow::bail!("{field_name} requires numeric value"),
        None => Ok(None),
    }
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
            if let Some(value) = parse_numeric_value(cmd.value.as_ref(), true, "volume_set")? {
                run_playerctl(player, &["volume", &value.to_string()])?;
            }
        }
        "volume_up" => {
            if let Some(value) = parse_numeric_value(cmd.value.as_ref(), false, "volume_up")? {
                run_playerctl(player, &["volume", &format!("+{}", value)])?;
            } else {
                run_playerctl(player, &["volume", "+0.05"])?;
            }
        }
        "volume_down" => {
            if let Some(value) = parse_numeric_value(cmd.value.as_ref(), false, "volume_down")? {
                run_playerctl(player, &["volume", &format!("-{}", value)])?;
            } else {
                run_playerctl(player, &["volume", "-0.05"])?;
            }
        }
        "mute" => {
            run_playerctl(player, &["volume", "0"])?;
        }
        "position_set" => {
            if let Some(value) = parse_numeric_value(cmd.value.as_ref(), true, "position_set")? {
                run_playerctl(player, &["position", &value.to_string()])?;
            }
        }
        "position_seek" => {
            if let Some(value) = parse_numeric_value(cmd.value.as_ref(), true, "position_seek")? {
                if value >= 0.0 {
                    run_playerctl(player, &["position", &format!("+{}", value)])?;
                } else {
                    run_playerctl(player, &["position", &value.to_string()])?;
                }
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{parse_command, parse_numeric_value};

    #[test]
    fn parse_command_accepts_action_and_value() {
        let cmd = parse_command(r#"{"action":"volume_set","value":0.6}"#)
            .expect("valid command should parse");
        assert_eq!(cmd.action, "volume_set");
        assert_eq!(cmd.value, Some(json!(0.6)));
    }

    #[test]
    fn parse_command_rejects_invalid_json() {
        let err = parse_command("not-json").expect_err("invalid JSON must fail");
        assert!(err.to_string().contains("valid JSON"));
    }

    #[test]
    fn parse_numeric_value_validates_required_numbers() {
        let ok = parse_numeric_value(Some(&json!(1.25)), true, "position_set")
            .expect("number should pass");
        assert_eq!(ok, Some(1.25));

        let missing_optional = parse_numeric_value(None, false, "volume_up")
            .expect("optional missing should pass");
        assert_eq!(missing_optional, None);

        let err = parse_numeric_value(Some(&json!("loud")), true, "volume_set")
            .expect_err("required non-number should fail");
        assert!(err.to_string().contains("volume_set requires numeric value"));
    }
}
