// See the "macOS permissions note" in README.md before running this on macOS
// Big Sur or later.

mod ipc;

use btleplug::api::WriteType;
use btleplug::api::{
    Central, CentralEvent, Manager as _, Peripheral, ScanFilter, bleuuid::BleUuid,
};
use btleplug::platform::{Adapter, Manager};
use common::weather::WeatherPayload;
use common::{MAX_PAYLOAD_LEN, Message, ToBoard, display_service_uuid, weather_payload_uuid};
use futures::stream::StreamExt;
use std::time::Duration;
use tokio::sync::watch;

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
    println!("Connected");

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
        println!("Sent payload ({} bytes)", bytes.len());
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let (tx, mut latest) = watch::channel(None);
    let ipc = IpcService::default(tx).await;
    tokio::spawn(ipc.run());
    println!("ipc");

    let manager = Manager::new().await?;

    // get the first bluetooth adapter
    // connect to the adapter
    let central = get_central(&manager).await;

    let central_state = central.adapter_state().await.unwrap();
    println!("CentralState: {:?}", central_state);

    // Each adapter has an event stream, we fetch via events(),
    // simplifying the type, this will return what is essentially a
    // Future<Result<Stream<Item=CentralEvent>>>.
    let mut events = central.events().await?;

    // start scanning for devices
    central
        .start_scan(ScanFilter {
            services: vec![display_service_uuid()],
        })
        .await?;

    // Process events asynchronously. In a real program, run this loop in its own
    // Tokio task if event handling should proceed independently of other work.
    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) => {
                let peripheral = central.peripheral(&id).await?;
                let properties = peripheral.properties().await?;
                let name = properties
                    .and_then(|p| p.local_name)
                    .map(|local_name| format!("Name: {local_name}"))
                    .unwrap_or_default();
                println!("DeviceDiscovered: {:?} {}", id, name);

                central.stop_scan().await?;
                loop {
                    if let Err(e) = forward(&peripheral, &mut latest).await {
                        println!("board link lost: {e:#}");
                        peripheral.disconnect().await.ok();
                        tokio::time::sleep(RECONNECT_DELAY).await;
                    }
                }
            }
            CentralEvent::StateUpdate(state) => {
                println!("AdapterStatusUpdate {:?}", state);
            }
            CentralEvent::DeviceConnected(id) => {
                println!("DeviceConnected: {:?}", id);
            }
            CentralEvent::DeviceDisconnected(id) => {
                println!("DeviceDisconnected: {:?}", id);
            }
            CentralEvent::ManufacturerDataAdvertisement {
                id,
                manufacturer_data,
            } => {
                println!(
                    "ManufacturerDataAdvertisement: {:?}, {:?}",
                    id, manufacturer_data
                );
            }
            CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                println!("ServiceDataAdvertisement: {:?}, {:?}", id, service_data);
            }
            CentralEvent::ServicesAdvertisement { id, services } => {
                let services: Vec<String> =
                    services.into_iter().map(|s| s.to_short_string()).collect();
                println!("ServicesAdvertisement: {:?}, {:?}", id, services);
            }
            _ => {}
        }
    }
    Ok(())
}
