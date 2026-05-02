use std::process::Command;

use anyhow::{Context, Result};

use crate::types::{Capabilities, CapabilityProbeCheck, CapabilityProbeReport, PlayerState};
use crate::util::{parse_f64, parse_mpris_length, parse_mpris_position, sanitize};

trait PlayerctlBackend {
    fn output(&self, player: &str, args: &[&str]) -> Result<String>;
}

struct SystemPlayerctl;

impl PlayerctlBackend for SystemPlayerctl {
    fn output(&self, player: &str, args: &[&str]) -> Result<String> {
        let output = Command::new("playerctl")
            .arg("--player")
            .arg(player)
            .args(args)
            .output()
            .with_context(|| format!("failed to run playerctl {:?}", args))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("playerctl {:?} failed: {}", args, stderr.trim());
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

fn run_playerctl_output(player: &str, args: &[&str]) -> Result<String> {
    SystemPlayerctl.output(player, args)
}

pub fn run_playerctl(player: &str, args: &[&str]) -> Result<()> {
    let status = Command::new("playerctl")
        .arg("--player")
        .arg(player)
        .args(args)
        .status()
        .context("failed to execute playerctl")?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("playerctl command failed")
    }
}

pub fn read_state(player: &str) -> Result<PlayerState> {
    let template = "{{status}}\t{{artist}}\t{{title}}\t{{album}}\t{{mpris:artUrl}}\t{{volume}}\t{{position}}\t{{mpris:length}}\t{{loop}}\t{{shuffle}}\t{{playerName}}";

    let out = run_playerctl_output(player, &["metadata", "--format", template])?;
    let parts: Vec<&str> = out.split('\t').collect();
    if parts.len() < 11 {
        anyhow::bail!("metadata output has unexpected format");
    }

    let state = parts[0].trim().to_lowercase();
    let artist = sanitize(parts[1].trim());
    let title = sanitize(parts[2].trim());
    let album = sanitize(parts[3].trim());
    let art_url = sanitize(parts[4].trim());

    let volume = parse_f64(parts[5]);
    let position_seconds = parse_mpris_position(parts[6]);
    let duration_seconds = parse_mpris_length(parts[7]);

    let loop_status = parts[8].trim().to_lowercase();
    let shuffle = parts[9].trim().to_lowercase();
    let player_name = sanitize(parts[10].trim());

    Ok(PlayerState {
        state,
        artist,
        title,
        album,
        art_url,
        volume,
        position_seconds,
        duration_seconds,
        loop_status,
        shuffle,
        player: player_name,
    })
}

fn parse_playerctl_bool(s: &str) -> Option<bool> {
    let normalized = s.trim().trim_matches('"').to_ascii_lowercase();
    match normalized.as_str() {
        "true" => Some(true),
        "1" => Some(true),
        "yes" => Some(true),
        "on" => Some(true),
        "false" => Some(false),
        "0" => Some(false),
        "no" => Some(false),
        "off" => Some(false),
        _ => None,
    }
}

fn is_missing_template_value(trimmed: &str, template: &str) -> bool {
    trimmed.is_empty() || trimmed == template
}

fn run_template<B: PlayerctlBackend>(backend: &B, player: &str, template: &str) -> Result<Option<String>> {
    let out = backend.output(player, &["metadata", "--format", template])?;
    let trimmed = out.trim();
    if is_missing_template_value(trimmed, template) {
        return Ok(None);
    }

    Ok(Some(trimmed.to_string()))
}

fn query_cap_bool<B: PlayerctlBackend>(backend: &B, player: &str, field: &str) -> (Option<bool>, String, String) {
    let templates = [
        format!("{{{{mpris:{field}}}}}"),
        format!("{{{{{field}}}}}"),
    ];
    let mut reasons = Vec::new();

    for template in templates {
        match run_template(backend, player, &template) {
            Ok(Some(out)) => {
                if let Some(parsed) = parse_playerctl_bool(&out) {
                    return (
                        Some(parsed),
                        "metadata".to_string(),
                        format!("{template}={out}"),
                    );
                }
                reasons.push(format!("{template} returned non-bool value '{out}'"));
            }
            Ok(None) => {
                reasons.push(format!("{template} unavailable"));
            }
            Err(err) => {
                reasons.push(format!("{template} error: {}", sanitize(&err.to_string())));
            }
        }
    }

    (
        None,
        "metadata".to_string(),
        reasons.join("; "),
    )
}

fn probe_has_value<B: PlayerctlBackend>(backend: &B, player: &str, template: &str) -> (bool, String) {
    match run_template(backend, player, template) {
        Ok(Some(value)) => (true, format!("{template}={value}")),
        Ok(None) => (false, format!("{template} unavailable")),
        Err(err) => (
            false,
            format!("{template} error: {}", sanitize(&err.to_string())),
        ),
    }
}

fn detect_capabilities_with_backend<B: PlayerctlBackend>(backend: &B, player: &str) -> (Capabilities, CapabilityProbeReport) {
    let status = backend.output(player, &["status"]);
    let status_value = match status {
        Ok(value) => value,
        Err(err) => {
            let report = CapabilityProbeReport::unavailable(
                player,
                &format!("status query failed: {}", sanitize(&err.to_string())),
            );
            return (Capabilities::unavailable(), report);
        }
    };

    let resolved_player = run_template(backend, player, "{{playerName}}")
        .ok()
        .flatten()
        .map(|name| sanitize(name.trim()));

    let mut checks = Vec::new();

    checks.push(CapabilityProbeCheck {
        capability: "can_control".to_string(),
        passed: true,
        source: "status".to_string(),
        reason: format!("status={}", status_value.trim()),
    });

    let (can_play_raw, can_play_source, can_play_reason) = query_cap_bool(backend, player, "canPlay");
    let can_play = can_play_raw.unwrap_or(true);
    checks.push(CapabilityProbeCheck {
        capability: "can_play".to_string(),
        passed: can_play,
        source: if can_play_raw.is_some() {
            can_play_source
        } else {
            "fallback".to_string()
        },
        reason: if can_play_raw.is_some() {
            can_play_reason
        } else {
            format!("fallback to can_control=true; {can_play_reason}")
        },
    });

    let (can_pause_raw, can_pause_source, can_pause_reason) = query_cap_bool(backend, player, "canPause");
    let can_pause = can_pause_raw.unwrap_or(true);
    checks.push(CapabilityProbeCheck {
        capability: "can_pause".to_string(),
        passed: can_pause,
        source: if can_pause_raw.is_some() {
            can_pause_source
        } else {
            "fallback".to_string()
        },
        reason: if can_pause_raw.is_some() {
            can_pause_reason
        } else {
            format!("fallback to can_control=true; {can_pause_reason}")
        },
    });

    let (can_next_raw, can_next_source, can_next_reason) = query_cap_bool(backend, player, "canGoNext");
    let can_next = can_next_raw.unwrap_or(true);
    checks.push(CapabilityProbeCheck {
        capability: "can_next".to_string(),
        passed: can_next,
        source: if can_next_raw.is_some() {
            can_next_source
        } else {
            "fallback".to_string()
        },
        reason: if can_next_raw.is_some() {
            can_next_reason
        } else {
            format!("fallback to can_control=true; {can_next_reason}")
        },
    });

    let (can_previous_raw, can_previous_source, can_previous_reason) =
        query_cap_bool(backend, player, "canGoPrevious");
    let can_previous = can_previous_raw.unwrap_or(true);
    checks.push(CapabilityProbeCheck {
        capability: "can_previous".to_string(),
        passed: can_previous,
        source: if can_previous_raw.is_some() {
            can_previous_source
        } else {
            "fallback".to_string()
        },
        reason: if can_previous_raw.is_some() {
            can_previous_reason
        } else {
            format!("fallback to can_control=true; {can_previous_reason}")
        },
    });

    let (can_seek_raw, can_seek_source, can_seek_reason) = query_cap_bool(backend, player, "canSeek");
    let can_seek = if let Some(parsed) = can_seek_raw {
        checks.push(CapabilityProbeCheck {
            capability: "can_seek".to_string(),
            passed: parsed,
            source: can_seek_source,
            reason: can_seek_reason,
        });
        parsed
    } else {
        let (has_position, position_reason) = probe_has_value(backend, player, "{{position}}");
        let (has_length, length_reason) = probe_has_value(backend, player, "{{mpris:length}}");
        let fallback = has_position || has_length;
        checks.push(CapabilityProbeCheck {
            capability: "can_seek".to_string(),
            passed: fallback,
            source: "probe".to_string(),
            reason: format!(
                "fallback by metadata value probe; {can_seek_reason}; {position_reason}; {length_reason}"
            ),
        });
        fallback
    };

    let can_set_volume = backend.output(player, &["volume"]).is_ok();
    checks.push(CapabilityProbeCheck {
        capability: "can_set_volume".to_string(),
        passed: can_set_volume,
        source: "query".to_string(),
        reason: if can_set_volume {
            "playerctl volume succeeded".to_string()
        } else {
            "playerctl volume failed".to_string()
        },
    });

    let can_shuffle = backend.output(player, &["shuffle"]).is_ok();
    checks.push(CapabilityProbeCheck {
        capability: "can_shuffle".to_string(),
        passed: can_shuffle,
        source: "query".to_string(),
        reason: if can_shuffle {
            "playerctl shuffle succeeded".to_string()
        } else {
            "playerctl shuffle failed".to_string()
        },
    });

    let can_loop = backend.output(player, &["loop"]).is_ok();
    checks.push(CapabilityProbeCheck {
        capability: "can_loop".to_string(),
        passed: can_loop,
        source: "query".to_string(),
        reason: if can_loop {
            "playerctl loop succeeded".to_string()
        } else {
            "playerctl loop failed".to_string()
        },
    });

    let (can_stop_raw, can_stop_source, can_stop_reason) = query_cap_bool(backend, player, "canStop");
    let can_stop = if let Some(parsed) = can_stop_raw {
        checks.push(CapabilityProbeCheck {
            capability: "can_stop".to_string(),
            passed: parsed,
            source: can_stop_source,
            reason: can_stop_reason,
        });
        parsed
    } else {
        let stopped = status_value.trim().eq_ignore_ascii_case("stopped");
        let fallback = can_pause || can_play || !stopped;
        checks.push(CapabilityProbeCheck {
            capability: "can_stop".to_string(),
            passed: fallback,
            source: "fallback".to_string(),
            reason: format!(
                "non-destructive fallback using can_play/can_pause/status; status={}; {}",
                status_value.trim(),
                can_stop_reason
            ),
        });
        fallback
    };

    let capabilities = Capabilities {
        can_play,
        can_pause,
        can_stop,
        can_next,
        can_previous,
        can_seek,
        can_set_volume,
        can_shuffle,
        can_loop,
    };

    let report = CapabilityProbeReport {
        status: "capabilities-probe".to_string(),
        player_selector: player.to_string(),
        resolved_player,
        fallback: false,
        checks,
        capabilities: capabilities.clone(),
    };

    (capabilities, report)
}

pub fn detect_capabilities_with_probe(player: &str) -> (Capabilities, CapabilityProbeReport) {
    detect_capabilities_with_backend(&SystemPlayerctl, player)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use anyhow::Result;

    use super::{
        PlayerctlBackend, detect_capabilities_with_backend, is_missing_template_value,
        parse_playerctl_bool,
    };

    struct FakePlayerctl {
        responses: HashMap<String, std::result::Result<String, String>>,
    }

    impl FakePlayerctl {
        fn key(player: &str, args: &[&str]) -> String {
            format!("{}|{}", player, args.join("\u{1f}"))
        }

        fn with(mut self, player: &str, args: &[&str], value: Result<&str>) -> Self {
            let stored = value
                .map(|v| v.to_string())
                .map_err(|err| err.to_string());
            self.responses.insert(Self::key(player, args), stored);
            self
        }
    }

    impl Default for FakePlayerctl {
        fn default() -> Self {
            Self {
                responses: HashMap::new(),
            }
        }
    }

    impl PlayerctlBackend for FakePlayerctl {
        fn output(&self, player: &str, args: &[&str]) -> Result<String> {
            let stored = self
                .responses
                .get(&Self::key(player, args))
                .cloned()
                .unwrap_or_else(|| Err(format!("missing response for {:?}", args)));

            stored.map_err(|err| anyhow::anyhow!(err))
        }
    }

    #[test]
    fn parse_playerctl_bool_accepts_common_values() {
        assert_eq!(parse_playerctl_bool("true"), Some(true));
        assert_eq!(parse_playerctl_bool("1"), Some(true));
        assert_eq!(parse_playerctl_bool("On"), Some(true));
        assert_eq!(parse_playerctl_bool("false"), Some(false));
        assert_eq!(parse_playerctl_bool("0"), Some(false));
        assert_eq!(parse_playerctl_bool("off"), Some(false));
        assert_eq!(parse_playerctl_bool("unknown"), None);
    }

    #[test]
    fn missing_template_helper_detects_empty_and_echo() {
        assert!(is_missing_template_value("", "{{canPlay}}"));
        assert!(is_missing_template_value("{{canPlay}}", "{{canPlay}}"));
        assert!(!is_missing_template_value("true", "{{canPlay}}"));
    }

    #[test]
    fn can_stop_uses_non_destructive_fallback() {
        let backend = FakePlayerctl::default()
            .with("p", &["status"], Ok("Playing"))
            .with("p", &["metadata", "--format", "{{playerName}}"], Ok("spotify"))
            .with("p", &["metadata", "--format", "{{mpris:canPlay}}"], Ok("true"))
            .with("p", &["metadata", "--format", "{{mpris:canPause}}"], Ok("true"))
            .with("p", &["metadata", "--format", "{{mpris:canGoNext}}"], Ok("true"))
            .with("p", &["metadata", "--format", "{{mpris:canGoPrevious}}"], Ok("true"))
            .with("p", &["metadata", "--format", "{{mpris:canSeek}}"], Ok("false"))
            .with("p", &["metadata", "--format", "{{mpris:canStop}}"], Err(anyhow::anyhow!("unsupported")))
            .with("p", &["metadata", "--format", "{{canStop}}"], Ok("{{canStop}}"))
            .with("p", &["volume"], Ok("0.2"))
            .with("p", &["shuffle"], Ok("Off"))
            .with("p", &["loop"], Ok("None"));

        let (caps, report) = detect_capabilities_with_backend(&backend, "p");
        assert!(caps.can_stop);
        assert!(!report.fallback);
        let stop_check = report
            .checks
            .iter()
            .find(|c| c.capability == "can_stop")
            .expect("can_stop check exists");
        assert_eq!(stop_check.source, "fallback");
    }
}
