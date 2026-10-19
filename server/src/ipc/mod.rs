use std::sync::Arc;

use common::MAX_PAYLOAD_LEN;
use common::ipc::HUB_SOCKET;
use common::weather::WeatherPayload;
use futures::{SinkExt, StreamExt};
use tokio::net::{UnixListener, UnixStream};
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::{info, warn};

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

    fn payload(&self) -> WeatherPayload {
        let mut payload = WeatherPayload::default();
        let now = jiff::Timestamp::now().as_second();
        if let Some(daily) = self.weather.latest()
            && !fill_daily(&mut payload, &daily, self.weather.location(), now)
        {
            warn!(
                "not enough daily data: {} entries, first dt {:?}, now {now}",
                daily.data.len(),
                daily.data.first().map(|d| d.dt),
            );
        }
        if let Some(hourly) = self.weather.latest_hourly()
            && !fill_chart(&mut payload, &hourly, now)
        {
            warn!(
                "not enough hourly data for the chart: {} entries, first dt {:?}, last dt {:?}, now {now}",
                hourly.data.len(),
                hourly.data.first().map(|h| h.dt),
                hourly.data.last().map(|h| h.dt),
            );
        }
        payload
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
            info!("got {:?}", frame?);
            let bytes = self.payload().encode(buf)?;
            socket.send(Bytes::copy_from_slice(bytes)).await?;
        }

        Ok(())
    }
}
