use common::ipc::HUB_SOCKET;
use common::{MAX_PAYLOAD_LEN, Payload};
use futures::{SinkExt, StreamExt};
use tokio::net::UnixListener;
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
        let buf = &mut [0u8; MAX_PAYLOAD_LEN];
        let payload = Payload::new(1);
        loop {
            let (stream, _) = self.listener.accept().await?;
            let mut socket = Framed::new(stream, LengthDelimitedCodec::new());
            let bytes = payload.encode(buf)?;

            while let Some(frame) = socket.next().await {
                info!("got {:?}", frame?);
                socket.send(Bytes::copy_from_slice(bytes)).await?;
            }
        }
    }
}
