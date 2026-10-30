use btleplug::{
    api::{
        Central, CentralEvent, Manager as _, Peripheral, ScanFilter, WriteType, bleuuid::BleUuid,
    },
    platform::{Adapter, Manager},
};
use common::{
    MAX_PAYLOAD_LEN, Message, ToBoard, display_service_uuid, weather::WeatherPayload,
    weather_payload_uuid,
};
use futures::StreamExt;
use std::time::Duration;
use tokio::sync::watch::{self, Receiver};
use tracing::{info, warn};

const RECONNECT_DELAY: Duration = Duration::from_secs(5);

pub struct BleService {
    central: Adapter,
    latest: Receiver<Option<WeatherPayload>>,
}

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

impl BleService {
    pub fn new(central: Adapter, latest: watch::Receiver<Option<WeatherPayload>>) -> Self {
        Self { central, latest }
    }

    pub async fn default(latest: watch::Receiver<Option<WeatherPayload>>) -> anyhow::Result<Self> {
        let manager = Manager::new().await?;
        let central = get_central(&manager).await;

        Ok(BleService::new(central, latest))
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let mut events = self.central.events().await?;

        // start scanning for devices
        let filter = ScanFilter {
            services: vec![display_service_uuid()],
        };
        self.central.start_scan(filter.clone()).await?;

        // Process events asynchronously. In a real program, run this loop in its own
        // Tokio task if event handling should proceed independently of other work.
        while let Some(event) = events.next().await {
            match event {
                CentralEvent::DeviceDiscovered(id) | CentralEvent::DeviceUpdated(id) => {
                    let Ok(peripheral) = self.central.peripheral(&id).await else {
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

                    self.central.stop_scan().await?;
                    if let Err(e) = forward(&peripheral, &mut self.latest).await {
                        warn!("board link lost: {e:#}");
                        peripheral.disconnect().await.ok();
                        tokio::time::sleep(RECONNECT_DELAY).await;
                        self.central.start_scan(filter.clone()).await?;
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
        }

        Ok(())
    }
}
