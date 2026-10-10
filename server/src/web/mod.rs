use std::sync::Arc;

use askama::Template;
use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::Html,
    routing::get,
};
use tower_http::services::ServeDir;

use crate::weather::{self, Ip, WeatherInfo, service::WeatherService};

type WebResult<T> = Result<T, (StatusCode, String)>;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    ip: Ip,
    weather_info: Arc<WeatherInfo>,
}

#[derive(Template)]
#[template(path = "weather.html")]
struct WeatherTemplate {
    ip: Ip,
    weather_info: Arc<WeatherInfo>,
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

fn latest(weather: &WeatherService) -> WebResult<Arc<WeatherInfo>> {
    weather.latest().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "weather not available yet".to_string(),
    ))
}

async fn index(State(weather): State<Arc<WeatherService>>) -> WebResult<Html<String>> {
    let ip = weather::weather_test().await.map_err(bad_gateway)?;
    let weather_info = latest(&weather)?;
    render(IndexTemplate { ip, weather_info })
}

async fn weather_fragment(State(weather): State<Arc<WeatherService>>) -> WebResult<Html<String>> {
    let ip = weather::weather_test().await.map_err(bad_gateway)?;
    let weather_info = latest(&weather)?;
    render(WeatherTemplate { ip, weather_info })
}

async fn weather_json() -> WebResult<Json<Ip>> {
    weather::weather_test().await.map(Json).map_err(bad_gateway)
}
