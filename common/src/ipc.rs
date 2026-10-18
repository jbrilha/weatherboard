use serde::{Deserialize, Serialize};

use crate::weather::WeatherPayload;

pub const HUB_SOCKET: &str = "/tmp/weatherboard.sock";

#[derive(Serialize, Deserialize)]
pub struct Config {
    byte: u8,
}

#[derive(Serialize, Deserialize)]
pub enum ToServer {
    GetWeather,
    Config(Config),
}

#[derive(Serialize, Deserialize)]
pub enum ToHub {
    Weather(WeatherPayload),
    Config(Config),
}
