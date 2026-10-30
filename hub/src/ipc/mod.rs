use std::time::Duration;

use common::ipc::{HUB_SOCKET, ToHub, ToServer};
use common::weather::WeatherPayload;
use common::{MAX_PAYLOAD_LEN, Message};
use futures::SinkExt;
use futures::stream::StreamExt;
use tokio::net::UnixStream;
use tokio::sync::watch;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::{info, warn};

const RETRY_TIMEOUT: u64 = 30;

type FramedSock = Framed<UnixStream, LengthDelimitedCodec>;

pub struct IpcService {
    socket: Option<FramedSock>,
    latest: watch::Sender<Option<WeatherPayload>>,
}

impl IpcService {
    pub fn new(socket: Option<FramedSock>, latest: watch::Sender<Option<WeatherPayload>>) -> Self {
        Self {
            socket: socket,
            latest,
        }
    }

    pub async fn default(latest: watch::Sender<Option<WeatherPayload>>) -> Self {
        let socket = try_connect().await;
        IpcService::new(socket, latest)
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            match self.socket {
                Some(ref mut socket) => {
                    match request(socket).await {
                        Ok(frame) => match ToHub::decode(&frame) {
                            Ok(ToHub::Weather(payload)) => {
                                self.latest.send_if_modified(|current| {
                                    let changed = *current != Some(payload);
                                    *current = Some(payload);
                                    changed
                                });
                            }
                            Ok(ToHub::Config(config)) => {
                                warn!("config not handled yet: {config:?}")
                            }
                            Ok(ToHub::WeatherUnavailable) => {
                                info!("server has no weather yet, keeping the last payload")
                            }
                            Err(e) => warn!("reply rejected: {e:?}"),
                        },
                        Err(e) => {
                            warn!("lost server connection: {e:#}");
                            self.socket = None;
                            continue;
                        }
                    }
                    tokio::time::sleep(Duration::from_secs(10)).await;
                }
                None => {
                    warn!(
                        "no socket available, trying again in {} seconds",
                        RETRY_TIMEOUT
                    );
                    tokio::time::sleep(Duration::from_secs(RETRY_TIMEOUT)).await;
                    self.socket = try_connect().await;
                }
            }
        }
    }
}

async fn request(socket: &mut FramedSock) -> anyhow::Result<BytesMut> {
    let mut buf = [0u8; MAX_PAYLOAD_LEN];
    let bytes = ToServer::GetWeather.encode(&mut buf)?;
    socket.send(Bytes::copy_from_slice(bytes)).await?;
    let frame = socket
        .next()
        .await
        .ok_or_else(|| anyhow::anyhow!("server closed the socket"))??;
    Ok(frame)
}

async fn try_connect() -> Option<FramedSock> {
    match UnixStream::connect(HUB_SOCKET).await {
        Ok(stream) => {
            info!("connected to socket");
            Some(Framed::new(stream, LengthDelimitedCodec::new()))
        }
        Err(_) => {
            warn!("failed to connect to socket");
            None
        }
    }
}
