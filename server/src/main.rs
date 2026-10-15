mod ipc;
mod weather;
mod web;

use std::{path::Path, sync::Arc, time::Duration};

use common::ipc::HUB_SOCKET;
use futures::{SinkExt, StreamExt};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tower_http::trace::TraceLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

use weather::{WeatherRequest, client::WeatherClient, service::WeatherService};

use crate::ipc::IpcService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
        )
        .init();

    let ipc = IpcService::default()?;
    tokio::spawn(ipc.run());

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
