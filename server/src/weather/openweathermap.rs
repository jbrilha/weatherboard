use serde::{Deserialize, Serialize};

pub const ONE_CALL_API: &str = "https://api.openweathermap.org/data/4.0/onecall/";

#[derive(Clone, Serialize, Deserialize)]
pub enum OpenCallEndpoint {
    Current,
    Timeline1Minute,
    Timeline15Minutes,
    Timeline1Hour,
    Timeline1Day,
}

impl OpenCallEndpoint {
    pub const fn path(&self) -> &'static str {
        match self {
            OpenCallEndpoint::Current => "current",
            OpenCallEndpoint::Timeline1Minute => "timeline/1min",
            OpenCallEndpoint::Timeline15Minutes => "timeline/15min",
            OpenCallEndpoint::Timeline1Hour => "timeline/1h",
            OpenCallEndpoint::Timeline1Day => "timeline/1day",
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Temperature {
    pub day: f32,
    pub min: f32,
    pub max: f32,
    night: f32,
    eve: f32,
    morn: f32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FeelsLike {
    pub day: f32,
    night: f32,
    eve: f32,
    morn: f32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum WeatherMain {
    Ash,
    Clear,
    Clouds,
    Drizzle,
    Dust,
    Fog,
    Haze,
    Mist,
    Rain,
    Sand,
    Smoke,
    Snow,
    Squall,
    Thunderstorm,
    Tornado,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Weather {
    pub id: i32,
    pub main: WeatherMain,
    description: String,
    icon: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DailyForecast {
    pub dt: i64,
    pub sunrise: i64,
    pub sunset: i64,
    moonrise: i32,
    moonset: i32,
    moon_phase: f32,
    pub temp: Temperature,
    pub feels_like: FeelsLike,
    pub pressure: f32,
    pub humidity: i32,
    pub dew_point: f32,
    pub wind_speed: f32,
    wind_deg: i32,
    pub weather: Vec<Weather>,
    clouds: i32,
    #[serde(default)]
    pub pop: Option<f32>,
    #[serde(default)]
    pub uvi: Option<f32>,
    #[serde(default)]
    rain: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HourlyForecast {
    pub dt: i64,
    pub temp: f32,
    feels_like: f32,
    pressure: f32,
    humidity: i32,
    dew_point: f32,
    #[serde(default)]
    uvi: Option<f32>,
    clouds: i32,
    wind_speed: f32,
    wind_deg: i32,
    weather: Vec<Weather>,
    #[serde(default)]
    pub pop: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct QuarterHourlyForecast {
    dt: i32,
    temp: Temperature,
    feels_like: FeelsLike,
    pressure: f32,
    humidity: i32,
    dew_point: f32,
    #[serde(default)]
    uvi: Option<f32>,
    wind_speed: f32,
    wind_deg: i32,
    #[serde(default)]
    pop: Option<f32>,
    weather: Vec<Weather>,
    clouds: i32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Forecast<T> {
    pub lat: f32,
    pub lon: f32,
    pub timezone: String,
    pub timezone_offset: i32,
    pub data: Vec<T>,
    #[serde(default, skip_serializing)]
    pub next: Option<String>,
}

impl<T: Serialize> Forecast<T> {
    pub fn pretty_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

pub type Forecast1Day = Forecast<DailyForecast>;
pub type Forecast1Hour = Forecast<HourlyForecast>;
pub type Forecast15Min = Forecast<HourlyForecast>;

#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Units {
    Standard,
    #[default]
    Metric,
    Imperial,
}
