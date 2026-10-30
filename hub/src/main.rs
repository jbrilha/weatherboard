mod ble;
mod ipc;
mod mqtt;

use tokio::sync::watch;

use crate::ble::BleService;
use crate::ipc::IpcService;
use crate::mqtt::MqttService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let (tx, rx) = watch::channel(None);
    // let ipc = IpcService::default(tx).await;

    let mqtt = MqttService::default(tx).await;

    let ble = BleService::default(rx).await?;

    tokio::try_join!(
        // ipc.run(),
        mqtt.run(),
        ble.run()
    )?;
    Ok(())
}
