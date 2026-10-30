// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

mod ipc;

use btleplug::api::WriteType;
use btleplug::api::{
    Central, CentralEvent, Manager as _, Peripheral, ScanFilter, bleuuid::BleUuid,
};
use btleplug::platform::{Adapter, Manager};
use common::mqtt::WEATHER_TOPIC;
use common::weather::WeatherPayload;
use common::{MAX_PAYLOAD_LEN, Message, ToBoard, display_service_uuid, weather_payload_uuid};
use futures::stream::StreamExt;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::time::Duration;
use tokio::sync::watch;
use tokio::{task, time};
use tracing::{error, info, warn};

use crate::ipc::IpcService;

const RECONNECT_DELAY: Duration = Duration::from_secs(5);

async fn get_central(manager: &Manager) -> Adapter {
    let adapters = manager.adapters().await.unwrap();
    adapters.into_iter().next().unwrap()
}

async fn forward(
    peripheral: &impl Peripheral,
    latest: &mut watch::Receiver<Option<WeatherPayload>>,
) -> anyhow::Result<()> {
    peripheral.connect().await?;
    info!("Connected");

    peripheral.discover_services().await?;
    let ch = peripheral
        .characteristics()
        .into_iter()
        .find(|c| c.uuid == weather_payload_uuid())
        .ok_or(anyhow::anyhow!("payload characteristic not found"))?;

    let mut buf = [0u8; MAX_PAYLOAD_LEN];
    latest.mark_changed();

    loop {
        latest.changed().await?;
        let Some(payload) = *latest.borrow_and_update() else {
            continue;
        };
        let bytes = ToBoard::Weather(payload).encode(&mut buf)?;
        peripheral
            .write(&ch, bytes, WriteType::WithResponse)
            .await?;
        info!("Sent payload ({} bytes)", bytes.len());
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let (tx, mut latest) = watch::channel(None);
    let ipc = IpcService::default(tx).await;
    tokio::spawn(ipc.run());

    /* let manager = Manager::new().await?;

    // get the first bluetooth adapter
    // connect to the adapter
    let central = get_central(&manager).await;

    let central_state = central.adapter_state().await.unwrap();
    info!("CentralState: {:?}", central_state);

    // Each adapter has an event stream, we fetch via events(),
    // simplifying the type, this will return what is essentially a
    // Future<Result<Stream<Item=CentralEvent>>>.
    let mut events = central.events().await?;

    // start scanning for devices
    let filter = ScanFilter {
        services: vec![display_service_uuid()],
    };
    central.start_scan(filter.clone()).await?;

    // Process events asynchronously. In a real program, run this loop in its own
    // Tokio task if event handling should proceed independently of other work.
    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                let Ok(peripheral) = central.peripheral(&id).await else {
                    continue;
                };
                let name = peripheral
                    .properties()
                    .await
                    .ok()
                    .flatten()
                    .and_then(|p| p.local_name)
                    .map(|local_name| format!("Name: {local_name}"))
                    .unwrap_or_default();
                info!("Found board: {:?} {}", id, name);

                central.stop_scan().await?;
                if let Err(e) = forward(&peripheral, &mut latest).await {
                    warn!("board link lost: {e:#}");
                    peripheral.disconnect().await.ok();
                    tokio::time::sleep(RECONNECT_DELAY).await;
                    central.start_scan(filter.clone()).await?;
                }
            }
            CentralEvent::StateUpdate(state) => {
                info!("AdapterStatusUpdate {:?}", state);
            }
            CentralEvent::DeviceConnected(id) => {
                info!("DeviceConnected: {:?}", id);
            }
            CentralEvent::DeviceDisconnected(id) => {
                warn!("DeviceDisconnected: {:?}", id);
            }
            CentralEvent::ManufacturerDataAdvertisement {
                id,
                manufacturer_data,
            } => {
                info!(
                    "ManufacturerDataAdvertisement: {:?}, {:?}",
                    id, manufacturer_data
                );
            }
            CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                info!("ServiceDataAdvertisement: {:?}, {:?}", id, service_data);
            }
            CentralEvent::ServicesAdvertisement { id, services } => {
                let services: Vec<String> =
                    services.into_iter().map(|s| s.to_short_string()).collect();
                info!("ServicesAdvertisement: {:?}, {:?}", id, services);
            }
            _ => {}
        }
    } */

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
