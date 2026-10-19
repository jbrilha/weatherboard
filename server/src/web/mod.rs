use std::sync::Arc;

use askama::Template;
use axum::{Json, Router, extract::State, http::StatusCode, response::Html, routing::get};
use tower_http::services::ServeDir;

use crate::weather::{self, Ip, openweathermap::Forecast1Day, service::WeatherService};

type WebResult<T> = Result<T, (StatusCode, String)>;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    ip: Ip,
    forecast: Arc<Forecast1Day>,
}

#[derive(Template)]
#[template(path = "weather.html")]
struct WeatherTemplate {
    ip: Ip,
    forecast: Arc<Forecast1Day>,
}

pub fn router() -> Router<Arc<WeatherService>> {
    Router::new()
        .route("/", get(index))
        .route("/fragments/weather", get(weather_fragment))
        .route("/weather", get(weather_json))
        .nest_service(
            "/static",
            ServeDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/static")),
        )
}

fn render<T: Template>(template: T) -> WebResult<Html<String>> {
    template
        .render()
        .map(Html)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

fn bad_gateway(err: anyhow::Error) -> (StatusCode, String) {
    (StatusCode::BAD_GATEWAY, err.to_string())
}

fn latest(weather: &WeatherService) -> WebResult<Arc<Forecast1Day>> {
    weather.latest().map(|s| s.daily.clone()).ok_or_else(|| {
        let reason = weather
            .last_error()
            .unwrap_or_else(|| "first fetch in progress".to_string());
        (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("weather not available yet: {reason}"),
        )
    })
}

async fn index(State(weather): State<Arc<WeatherService>>) -> WebResult<Html<String>> {
    let ip = weather::weather_test().await.map_err(bad_gateway)?;
    let forecast = latest(&weather)?;
    render(IndexTemplate { ip, forecast })
}

async fn weather_fragment(State(weather): State<Arc<WeatherService>>) -> WebResult<Html<String>> {
    let ip = weather::weather_test().await.map_err(bad_gateway)?;
    let forecast = latest(&weather)?;
    render(WeatherTemplate { ip, forecast })
}

async fn weather_json() -> WebResult<Json<Ip>> {
    weather::weather_test().await.map(Json).map_err(bad_gateway)
}
