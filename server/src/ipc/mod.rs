use std::sync::Arc;

use common::ipc::{HUB_SOCKET, ToHub, ToServer};
use common::{MAX_PAYLOAD_LEN, Message};
use futures::{SinkExt, StreamExt};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::warn;

use crate::weather::service::WeatherService;

pub struct IpcService {
    listener: UnixListener,
    weather: Arc<WeatherService>,
}

impl IpcService {
    pub fn new(listener: UnixListener, weather: Arc<WeatherService>) -> Self {
        Self { listener, weather }
    }

    pub fn default(weather: Arc<WeatherService>) -> anyhow::Result<Self> {
        let _ = std::fs::remove_file(HUB_SOCKET);
        let listener = UnixListener::bind(HUB_SOCKET)?;
        Ok(IpcService::new(listener, weather))
    }

    pub async fn run(self) -> anyhow::Result<()> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            if let Err(e) = self.serve(stream).await {
                warn!("hub connection dropped: {e:#}");
            }
        }
    }

    async fn serve(&self, stream: UnixStream) -> anyhow::Result<()> {
        let buf = &mut [0u8; MAX_PAYLOAD_LEN];
        let mut socket = Framed::new(stream, LengthDelimitedCodec::new());

        while let Some(frame) = socket.next().await {
            match ToServer::decode(&frame?) {
                Ok(ToServer::GetWeather) => {
                    let reply = match self.weather.payload() {
                        Some(payload) => ToHub::Weather(payload),
                        None => ToHub::WeatherUnavailable,
                    };
                    let bytes = reply.encode(buf)?;
                    socket.send(Bytes::copy_from_slice(bytes)).await?;
                }
                Ok(ToServer::Config(config)) => warn!("config not handled yet: {config:?}"),
                Err(e) => warn!("request rejected: {e:?}"),
            }
        }

        Ok(())
    }
}
