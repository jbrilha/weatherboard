pub mod client;
pub mod service;

use serde::{Deserialize, Serialize};

use crate::weather::Units::Metric;

const ONE_CALL_API: &str = "https://api.openweathermap.org/data/4.0/onecall/";

#[derive(Serialize, Deserialize)]
pub struct Ip {
    pub origin: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Temperature {
    day: f32,
    min: f32,
    max: f32,
    night: f32,
    eve: f32,
    morn: f32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FeelsLike {
    day: f32,
    night: f32,
    eve: f32,
    morn: f32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Weather {
    id: i32,
    main: String,
    description: String,
    icon: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DailyForecast {
    dt: i32,
    sunrise: i32,
    sunset: i32,
    moonrise: i32,
    moonset: i32,
    moon_phase: f32,
    temp: Temperature,
    feels_like: FeelsLike,
    pressure: f32,
    humidity: i32,
    dew_point: f32,
    wind_speed: f32,
    wind_deg: i32,
    weather: Vec<Weather>,
    clouds: i32,
    pop: f32,
    #[serde(default)]
    uvi: Option<f32>,
    #[serde(default)]
    rain: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Forecast {
    pub lat: f32,
    pub lon: f32,
    pub timezone: String,
    pub timezone_offset: i32,
    pub data: Vec<DailyForecast>,
}

impl Forecast {
    pub fn pretty_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
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
        WeatherRequest::new(38.7369, -9.142685, Some(Metric))
    }
}

pub async fn weather_test() -> anyhow::Result<Ip> {
    let response = reqwest::get("https://httpbin.org/ip")
        .await?
        .json::<Ip>()
        .await?;

    Ok(response)
}
