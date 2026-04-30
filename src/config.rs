use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "mpris-mqtt-adapter")]
#[command(about = "MPRIS/Playerctl to MQTT bridge")]
pub struct Cli {
    #[arg(long, default_value = "mqtt.home.arpa")]
    pub host: String,

    #[arg(long, default_value_t = 1883)]
    pub port: u16,

    #[arg(long, default_value = "workstation/media")]
    pub topic: String,

    #[arg(long, default_value_t = false)]
    pub discovery: bool,

    #[arg(long, default_value_t = 2)]
    pub poll_seconds: u64,

    #[arg(long, default_value = "playerctld,%any")]
    pub player: String,
}
