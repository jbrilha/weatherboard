pub mod client;
pub mod service;

use serde::{Deserialize, Serialize};

use crate::weather::Units::Metric;

const ONE_CALL_API: &str = "https://api.openweathermap.org/data/4.0/onecall/";

#[derive(Serialize, Deserialize)]
pub struct Ip {
    pub origin: String,
}

#[derive(Serialize, Deserialize)]
pub struct WeatherInfo {
    pub lat: f32,
    pub lon: f32,
    pub timezone: String,
    pub timezone_offset: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Units {
    Standard,
    Metric,
    Imperial,
}

#[derive(Serialize, Deserialize)]
pub struct WeatherRequest {
    pub lat: f32,
    pub lon: f32,
    #[serde(default)]
    pub units: Option<Units>,
}

impl WeatherRequest {
    pub fn new(lat: f32, lon: f32, units: Option<Units>) -> Self {
        Self { lat, lon, units }
    }

    pub fn lisbon() -> Self {
        Self {
            lat: 38.7369,
            lon: -9.142685,
            units: Some(Metric),
        }
    }
}

pub async fn weather_test() -> anyhow::Result<Ip> {
    let response = reqwest::get("https://httpbin.org/ip")
        .await?
        .json::<Ip>()
        .await?;

    Ok(response)
}
