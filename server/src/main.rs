mod ipc;
mod mqtt;
mod weather;
mod web;

use std::sync::Arc;

use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use weather::service::WeatherService;

use crate::{ipc::IpcService, mqtt::MqttService};

const HOST: &str = "127.0.0.1";
const PORT: &str = "3030";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .init();

    let weather = Arc::new(WeatherService::default());
    tokio::spawn(weather.clone().run());

    let ipc = IpcService::default(weather.clone())?;
    tokio::spawn(ipc.run());

    let mqtt = MqttService::default()?;

    let app = web::router()
        .with_state(weather)
        .layer(TraceLayer::new_for_http());
    #[cfg(feature = "livereload")]
    let app = app.layer(tower_livereload::LiveReloadLayer::new());

    let bind_addr = format!("{}:{}", HOST, PORT);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
