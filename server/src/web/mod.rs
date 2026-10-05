use askama::Template;
use axum::{Json, Router, http::StatusCode, response::Html, routing::get};
use tower_http::services::ServeDir;

use crate::weather::{self, Ip};

type WebResult<T> = Result<T, (StatusCode, String)>;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    ip: Ip,
}

#[derive(Template)]
#[template(path = "weather.html")]
struct WeatherTemplate {
    ip: Ip,
}

pub fn router() -> Router {
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

async fn index() -> WebResult<Html<String>> {
    let ip = weather::weather_test().await.map_err(bad_gateway)?;
    render(IndexTemplate { ip })
}

async fn weather_fragment() -> WebResult<Html<String>> {
    let ip = weather::weather_test().await.map_err(bad_gateway)?;
    render(WeatherTemplate { ip })
}

async fn weather_json() -> WebResult<Json<Ip>> {
    weather::weather_test()
        .await
        .map(Json)
        .map_err(bad_gateway)
}
