use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use common::ipc::HUB_SOCKET;
use futures::SinkExt;
use futures::stream::StreamExt;
use tokio::{
    net::{UnixListener, UnixStream},
    sync::watch,
};
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

type FramedSock = Framed<UnixStream, LengthDelimitedCodec>;

pub struct IpcService {
    socket: FramedSock,
}

impl IpcService {
    pub fn new(socket: FramedSock) -> Self {
        Self { socket }
    }

    pub async fn default() -> anyhow::Result<Self> {
        let stream = UnixStream::connect(HUB_SOCKET).await?;
        let socket = Framed::new(stream, LengthDelimitedCodec::new());
        Ok(IpcService::new(socket))
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        loop {
            self.socket.send(Bytes::from_static(b"ping")).await?;
            if let Some(frame) = self.socket.next().await {
                println!("got {:?}", frame?);
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }
    }
}
