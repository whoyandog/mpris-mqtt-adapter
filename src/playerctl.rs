use std::process::Command;

use anyhow::{Context, Result};

use crate::types::{Capabilities, PlayerState};
use crate::util::{parse_f64, parse_mpris_length, parse_mpris_position, sanitize};

fn run_playerctl_output(player: &str, args: &[&str]) -> Result<String> {
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

fn run_template(player: &str, template: &str) -> Option<String> {
    let out = run_playerctl_output(player, &["metadata", "--format", template]).ok()?;
    let trimmed = out.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Some unknown templates are echoed back as-is. Treat this as missing.
    if trimmed == template {
        return None;
    }

    Some(trimmed.to_string())
}

fn query_cap_bool(player: &str, field: &str) -> Option<bool> {
    let templates = [
        format!("{{{{mpris:{field}}}}}"),
        format!("{{{{{field}}}}}"),
    ];

    for template in templates {
        if let Some(out) = run_template(player, &template)
            && let Some(parsed) = parse_playerctl_bool(&out)
        {
            return Some(parsed);
        }
    }

    None
}

fn probe_has_value(player: &str, template: &str) -> bool {
    run_template(player, template).is_some()
}

pub fn detect_capabilities(player: &str) -> Result<Capabilities> {
    let can_control = run_playerctl_output(player, &["status"]).is_ok();

    if !can_control {
        anyhow::bail!("player not available");
    }

    let can_play = query_cap_bool(player, "canPlay").unwrap_or(can_control);
    let can_pause = query_cap_bool(player, "canPause").unwrap_or(can_control);
    let can_next = query_cap_bool(player, "canGoNext").unwrap_or(can_control);
    let can_previous = query_cap_bool(player, "canGoPrevious").unwrap_or(can_control);

    let can_seek = query_cap_bool(player, "canSeek")
        .unwrap_or_else(|| probe_has_value(player, "{{position}}") || probe_has_value(player, "{{mpris:length}}"));

    let can_set_volume = can_control && run_playerctl_output(player, &["volume"]).is_ok();
    let can_shuffle = can_control && run_playerctl_output(player, &["shuffle"]).is_ok();
    let can_loop = can_control && run_playerctl_output(player, &["loop"]).is_ok();
    let can_stop = query_cap_bool(player, "canControl").unwrap_or(can_control);

    Ok(Capabilities {
        can_play,
        can_pause,
        can_stop,
        can_next,
        can_previous,
        can_seek,
        can_set_volume,
        can_shuffle,
        can_loop,
    })
}
