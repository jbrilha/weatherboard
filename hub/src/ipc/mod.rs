use std::time::Duration;

use common::ipc::HUB_SOCKET;
use common::weather::WeatherPayload;
use futures::SinkExt;
use futures::stream::StreamExt;
use tokio::net::UnixStream;
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::{info, warn};

const RETRY_TIMEOUT: u64 = 30;

type FramedSock = Framed<UnixStream, LengthDelimitedCodec>;

pub struct IpcService {
    socket: Option<FramedSock>,
}

impl IpcService {
    pub fn new(socket: FramedSock) -> Self {
        Self {
            socket: Some(socket),
        }
    }

    pub async fn default() -> Self {
        let socket = try_connect().await;
        Self { socket }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            match self.socket {
                Some(ref mut socket) => {
                    socket.send(Bytes::from_static(b"ping")).await?;
                    if let Some(frame) = socket.next().await {
                        let p = WeatherPayload::decode(&frame?);
                        println!("got {:?}", p);
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
