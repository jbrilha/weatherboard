mod weather;
mod web;

use std::sync::Arc;

use tower_http::trace::TraceLayer;

use weather::{WeatherRequest, client::WeatherClient, service::WeatherService};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let weather = Arc::new(WeatherService::default());
    tokio::spawn(weather.clone().run());

    let app = web::router()
        .with_state(weather)
        .layer(TraceLayer::new_for_http());
    #[cfg(feature = "livereload")]
    let app = app.layer(tower_livereload::LiveReloadLayer::new());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
