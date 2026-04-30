use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, PartialEq)]
pub struct PlayerState {
    pub state: String,
    pub artist: String,
    pub title: String,
    pub album: String,
    pub art_url: String,
    pub volume: Option<f64>,
    pub position_seconds: Option<f64>,
    pub duration_seconds: Option<f64>,
    pub loop_status: String,
    pub shuffle: String,
    pub player: String,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
pub struct Capabilities {
    pub can_play: bool,
    pub can_pause: bool,
    pub can_stop: bool,
    pub can_next: bool,
    pub can_previous: bool,
    pub can_seek: bool,
    pub can_set_volume: bool,
    pub can_shuffle: bool,
    pub can_loop: bool,
}

impl Capabilities {
    pub fn unavailable() -> Self {
        Self {
            can_play: false,
            can_pause: false,
            can_stop: false,
            can_next: false,
            can_previous: false,
            can_seek: false,
            can_set_volume: false,
            can_shuffle: false,
            can_loop: false,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CmdMsg {
    pub action: String,
    pub value: Option<Value>,
}
