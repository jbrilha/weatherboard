use std::sync::Arc;

use common::ipc::{HUB_SOCKET, ToHub, ToServer};
use common::weather::WeatherPayload;
use common::{MAX_PAYLOAD_LEN, Message};
use futures::{SinkExt, StreamExt};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::warn;

use crate::weather::{
    payload::{fill_chart, fill_daily},
    service::WeatherService,
};

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

    fn payload(&self) -> Option<WeatherPayload> {
        let snapshot = self.weather.latest()?;
        let (daily, hourly) = (&snapshot.daily, &snapshot.hourly);
        let now = jiff::Timestamp::now().as_second();
        let mut payload = WeatherPayload::default();
        if !fill_daily(
            &mut payload,
            daily,
            self.weather.location(),
            now,
            snapshot.fetched_at,
        ) {
            warn!(
                "not enough daily data: {} entries, first dt {:?}, now {now}",
                daily.data.len(),
                daily.data.first().map(|d| d.dt),
            );
            return None;
        }
        if !fill_chart(&mut payload, hourly, now) {
            warn!(
                "not enough hourly data for the chart: {} entries, first dt {:?}, last dt {:?}, now {now}",
                hourly.data.len(),
                hourly.data.first().map(|h| h.dt),
                hourly.data.last().map(|h| h.dt),
            );
            return None;
        }
        Some(payload)
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
                    let reply = match self.payload() {
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
