// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

mod ble;
mod ipc;

use common::mqtt::WEATHER_TOPIC;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::time::Duration;
use tokio::sync::watch;
use tokio::{task, time};
use tracing::{error, info};

use crate::ble::BleService;
use crate::ipc::IpcService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let (tx, mut latest) = watch::channel(None);
    let ipc = IpcService::default(tx).await;
    tokio::spawn(ipc.run());

    let ble = BleService::default(latest).await?;
    tokio::spawn(ble.run());

    let mqttoptions = MqttOptions::new("test-1", "localhost", 1883);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    task::spawn(async move {
        requests(client).await;
        time::sleep(Duration::from_secs(3)).await;
    });

    loop {
        let event = eventloop.poll().await;
        match &event {
            Ok(v) => {
                info!("Event = {v:?}");
            }
            Err(e) => {
                error!("Error = {e:?}");
                return Ok(());
            }
        }
    }

    Ok(())
}

async fn requests(client: AsyncClient) {
    client
        .subscribe(WEATHER_TOPIC, QoS::AtMostOnce)
        .await
        .unwrap();

    /* for i in 1..=10 {
        client
            .publish("hello/world", QoS::ExactlyOnce, false, vec![1; i])
            .await
            .unwrap();

        time::sleep(Duration::from_secs(1)).await;
    } */

    time::sleep(Duration::from_secs(120)).await;
}
