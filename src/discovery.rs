use anyhow::Result;
use rumqttc::{AsyncClient, QoS};

pub async fn publish_discovery(
    client: &AsyncClient,
    base_topic: &str,
    state_topic: &str,
    cmd_topic: &str,
) -> Result<()> {
    let availability_topic = format!("{}/availability", base_topic);
    let device = serde_json::json!({
        "identifiers": ["workstation_media"],
        "name": "Workstation Media Adapter",
        "manufacturer": "custom",
        "model": "mpris-mqtt-adapter"
    });

    let discovery_entries: [(&str, serde_json::Value); 9] = [
        (
            "homeassistant/sensor/workstation_media_state/config",
            serde_json::json!({
                "name": "Workstation Media State",
                "unique_id": "workstation_media_state",
                "state_topic": state_topic,
                "value_template": "{{ value_json.state }}",
                "availability_topic": availability_topic,
                "payload_available": "online",
                "payload_not_available": "offline",
                "device": device
            }),
        ),
        (
            "homeassistant/sensor/workstation_media_title/config",
            serde_json::json!({
                "name": "Workstation Media Title",
                "unique_id": "workstation_media_title",
                "state_topic": state_topic,
                "value_template": "{{ value_json.title }}",
                "availability_topic": availability_topic,
                "payload_available": "online",
                "payload_not_available": "offline",
                "device": device
            }),
        ),
        (
            "homeassistant/sensor/workstation_media_artist/config",
            serde_json::json!({
                "name": "Workstation Media Artist",
                "unique_id": "workstation_media_artist",
                "state_topic": state_topic,
                "value_template": "{{ value_json.artist }}",
                "availability_topic": availability_topic,
                "payload_available": "online",
                "payload_not_available": "offline",
                "device": device
            }),
        ),
        (
            "homeassistant/sensor/workstation_media_album/config",
            serde_json::json!({
                "name": "Workstation Media Album",
                "unique_id": "workstation_media_album",
                "state_topic": state_topic,
                "value_template": "{{ value_json.album }}",
                "availability_topic": availability_topic,
                "payload_available": "online",
                "payload_not_available": "offline",
                "device": device
            }),
        ),
        (
            "homeassistant/sensor/workstation_media_art_url/config",
            serde_json::json!({
                "name": "Workstation Media Art URL",
                "unique_id": "workstation_media_art_url",
                "state_topic": state_topic,
                "value_template": "{{ value_json.art_url }}",
                "availability_topic": availability_topic,
                "payload_available": "online",
                "payload_not_available": "offline",
                "device": device
            }),
        ),
        (
            "homeassistant/button/workstation_media_play_pause/config",
            serde_json::json!({
                "name": "Workstation Media Play Pause",
                "unique_id": "workstation_media_play_pause",
                "command_topic": cmd_topic,
                "payload_press": "{\"action\":\"play_pause\"}",
                "availability_topic": availability_topic,
                "payload_available": "online",
                "payload_not_available": "offline",
                "device": device
            }),
        ),
        (
            "homeassistant/button/workstation_media_next/config",
            serde_json::json!({
                "name": "Workstation Media Next",
                "unique_id": "workstation_media_next",
                "command_topic": cmd_topic,
                "payload_press": "{\"action\":\"next\"}",
                "availability_topic": availability_topic,
                "payload_available": "online",
                "payload_not_available": "offline",
                "device": device
            }),
        ),
        (
            "homeassistant/button/workstation_media_previous/config",
            serde_json::json!({
                "name": "Workstation Media Previous",
                "unique_id": "workstation_media_previous",
                "command_topic": cmd_topic,
                "payload_press": "{\"action\":\"previous\"}",
                "availability_topic": availability_topic,
                "payload_available": "online",
                "payload_not_available": "offline",
                "device": device
            }),
        ),
        (
            "homeassistant/button/workstation_media_stop/config",
            serde_json::json!({
                "name": "Workstation Media Stop",
                "unique_id": "workstation_media_stop",
                "command_topic": cmd_topic,
                "payload_press": "{\"action\":\"stop\"}",
                "availability_topic": availability_topic,
                "payload_available": "online",
                "payload_not_available": "offline",
                "device": device
            }),
        ),
    ];

    for (topic, payload) in discovery_entries {
        client
            .publish(
                topic,
                QoS::AtLeastOnce,
                true,
                serde_json::to_vec(&payload)?,
            )
            .await?;
    }

    client
        .publish(
            format!("{}/availability", base_topic),
            QoS::AtLeastOnce,
            true,
            "online",
        )
        .await?;

    Ok(())
}
