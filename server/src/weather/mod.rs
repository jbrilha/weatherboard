pub mod client;
pub mod openweathermap;
pub mod payload;
pub mod service;

use common::weather::Location;
use serde::{Deserialize, Serialize};

use crate::weather::openweathermap::{OpenCallEndpoint, Units};

#[derive(Serialize, Deserialize)]
pub struct Ip {
    pub origin: String,
}

#[derive(Serialize, Deserialize)]
pub struct WeatherRequest {
    pub endpoint: OpenCallEndpoint,
    pub params: WeatherParams,
}

impl WeatherRequest {
    pub fn new(endpoint: OpenCallEndpoint, params: WeatherParams) -> Self {
        Self { endpoint, params }
    }
}

#[derive(Serialize, Deserialize)]
pub struct WeatherParams {
    pub lat: f32,
    pub lon: f32,
    #[serde(default)]
    pub units: Option<Units>,
}

impl WeatherParams {
    pub fn new(lat: f32, lon: f32, units: Option<Units>) -> Self {
        Self { lat, lon, units }
    }

    pub fn at(location: Location, units: Units) -> Self {
        let (lat, lon) = location.coords();
        Self {
            lat,
            lon,
            units: Some(units),
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
