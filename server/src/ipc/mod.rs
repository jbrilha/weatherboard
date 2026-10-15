use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use common::ipc::HUB_SOCKET;
use futures::{SinkExt, StreamExt};
use tokio::{net::UnixListener, sync::watch};
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec};
use tracing::info;

pub struct IpcService {
    listener: UnixListener,
}

impl IpcService {
    pub fn new(listener: UnixListener) -> Self {
        Self { listener }
    }

    pub fn default() -> anyhow::Result<Self> {
        let _ = std::fs::remove_file(HUB_SOCKET);
        let listener = UnixListener::bind(HUB_SOCKET)?;
        Ok(IpcService::new(listener))
    }

    pub async fn run(self) -> anyhow::Result<()> {
        loop {
            let (stream, _) = self.listener.accept().await?;
            let mut socket = Framed::new(stream, LengthDelimitedCodec::new());

            while let Some(frame) = socket.next().await {
                info!("got {:?}", frame?);
                socket.send(Bytes::from_static(b"pong")).await?;
            }
        }
    }
}
