mod ble;
#[cfg(feature = "ipc")]
mod ipc;
#[cfg(feature = "mqtt")]
mod mqtt;

#[cfg(all(feature = "mqtt", feature = "ipc"))]
compile_error!("mqtt XOR ipc");

use tokio::sync::watch;

use crate::ble::BleService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let (tx, rx) = watch::channel(None);
    #[cfg(feature = "mqtt")]
    let transport = mqtt::MqttService::default(tx).await;
    #[cfg(feature = "ipc")]
    let transport = ipc::IpcService::default(tx).await;
    let ble = BleService::default(rx).await?;

    tokio::try_join!(transport.run(), ble.run())?;
    Ok(())
}
