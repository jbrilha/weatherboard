use serde::{Deserialize, Serialize};

use crate::{Message, weather::WeatherPayload};

pub const HUB_SOCKET: &str = "/tmp/weatherboard.sock";

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    byte: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ToServer {
    GetWeather,
    Config(Config),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ToHub {
    Weather(WeatherPayload),
    Config(Config),
}

impl Message for ToServer {}
impl Message for ToHub {}
