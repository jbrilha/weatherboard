// embassy stm32wba65 ble_gatt_server example

use common::Payload;
use defmt::{Display2Format, debug, error, info, warn};
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_stm32_wpan::bluetooth::gap::types::OwnAddressType;
use embassy_stm32_wpan::bluetooth::gap::{AdvData, AdvParams, AdvType, GapEvent};
use embassy_stm32_wpan::bluetooth::gap_init::{AddressType, GapInitParams};
use embassy_stm32_wpan::bluetooth::gatt::{
    CHAR_VALUE_HANDLE_OFFSET, CccdValue, CharProperties, CharacteristicHandle, GattEventMask,
    SecurityPermissions, ServiceHandle, ServiceType, Uuid, is_cccd_handle, is_value_handle,
};
use embassy_stm32_wpan::bluetooth::{HCI, Normal};
use embassy_stm32_wpan::{Platform, new_platform};
use stm32wb_hci::Event;
use stm32wb_hci::vendor::event::{AttExchangeMtuResponse, VendorEvent};

use crate::board::Irqs;

// ---- Test configuration ----
const ADDR_TYPE: OwnAddressType = OwnAddressType::Random;

pub static PAYLOAD: Signal<CriticalSectionRawMutex, Payload> = Signal::new();

/// BLE runner task - drives the BLE stack sequencer
#[embassy_executor::task]
async fn ble_runner_task(platform: &'static Platform) {
    platform.run_ble().await
}

/// Application state for tracking GATT handles and CCCD state
struct AppState {
    service_handle: ServiceHandle,
    data_char_handle: CharacteristicHandle,
    notifications_enabled: bool,
    current_conn_handle: Option<u16>,
    counter: u8,
}

fn to_ble_uuid(uuid: common::Uuid) -> Uuid {
    Uuid::from_u128_le(uuid.as_u128().to_le_bytes())
}

pub async fn start(spawner: Spawner) {
    let (platform, runtime) = new_platform!(8);
    info!("BLE platform initialized");

    spawner.spawn(ble_runner_task(platform).expect("Failed to spawn BLE runner"));

    let ble = match ADDR_TYPE {
        OwnAddressType::Public => {
            let gap_params = GapInitParams {
                bd_addr: [0x01, 0x00, 0x00, 0xE1, 0x80, 0x00],
                address_type: AddressType::Public,
                ..GapInitParams::default()
            };
            HCI::new_with_gap_params(platform, runtime, Irqs, gap_params).await
        }
        _ => HCI::new(platform, runtime, Irqs).await,
    }
    .expect("BLE initialization failed");

    spawner.spawn(ble_task(ble).expect("Failed to spawn BLE task"));
}

#[embassy_executor::task]
async fn ble_task(mut ble: HCI<'static, Normal>) {
    // Initialize GATT server
    let mut gatt = ble.gatt_server();

    // Create custom service
    let service_uuid = to_ble_uuid(common::display_service_uuid());
    let service_handle = gatt
        .add_service(service_uuid, ServiceType::Primary, 6)
        .expect("Failed to add service");
    info!("Service created: handle 0x{:04X}", service_handle.0);

    // Add data characteristic with read/write/notify
    let char_uuid = to_ble_uuid(common::weather_payload_uuid());
    let char_properties = CharProperties::READ | CharProperties::WRITE | CharProperties::NOTIFY;
    let data_char_handle = gatt
        .add_characteristic(
            service_handle,
            char_uuid,
            32, // Max 32 bytes
            char_properties,
            SecurityPermissions::NONE,
            GattEventMask::ATTRIBUTE_MODIFIED,
            0,
            true, // Variable length
        )
        .expect("Failed to add characteristic");
    info!(
        "Characteristic created: handle 0x{:04X}",
        data_char_handle.0
    );
    info!(
        "  Value handle: 0x{:04X}",
        data_char_handle.0 + CHAR_VALUE_HANDLE_OFFSET
    );
    info!("  CCCD handle: 0x{:04X}", data_char_handle.0 + 2);

    // Set initial value
    let initial_value = b"Hello!";
    gatt.update_characteristic_value(service_handle, data_char_handle, 0, initial_value)
        .expect("Failed to set initial value");

    // Application state
    let mut state = AppState {
        service_handle,
        data_char_handle,
        notifications_enabled: false,
        current_conn_handle: None,
        counter: 0,
    };

    // Create advertising data
    let mut adv_data = AdvData::new();
    adv_data.add_flags(0x06).expect("Failed to add flags");
    adv_data.add_name("wboard").expect("Failed to add name");
    adv_data
        .add_service_uuid_128(&common::display_service_uuid().as_u128().to_le_bytes())
        .expect("Failed to add service UUID");

    let adv_params = AdvParams {
        interval_min: 0x0050,
        interval_max: 0x0050,
        adv_type: AdvType::ConnectableUndirected,
        own_addr_type: ADDR_TYPE,
        ..AdvParams::default()
    };

    // Start advertising
    {
        ble.start_advertising(adv_params.clone(), adv_data.clone(), None)
            .await
            .expect("Failed to start advertising");
    }

    info!("GATT Server started as 'Embassy-GATT'");
    info!("Waiting for connections...");

    // Main event loop
    loop {
        let event = ble.read_event().await;

        // Process GAP events (connections)
        if let Some(gap_event) = ble.process_event(&event) {
            match gap_event {
                GapEvent::Connected(conn) => {
                    info!("Connected: handle 0x{:04X}", conn.handle.0);
                    state.current_conn_handle = Some(conn.handle.0);
                    state.notifications_enabled = false; // Reset on new connection
                }
                GapEvent::Disconnected { handle, reason } => {
                    info!(
                        "Disconnected: handle 0x{:04X}, reason 0x{:02X} ({})",
                        handle.0,
                        reason.as_u8(),
                        Display2Format(&reason)
                    );
                    state.current_conn_handle = None;
                    state.notifications_enabled = false;

                    // Restart advertising
                    ble.start_advertising(adv_params.clone(), adv_data.clone(), None)
                        .await
                        .expect("Failed to start advertising");
                    info!("Advertising restarted");
                }
                _ => {}
            }
        }

        // Process GATT events
        match &event {
            Event::Vendor(VendorEvent::GattAttributeModified(attribute)) => {
                info!(
                    "Attribute modified: conn 0x{:04X}, attr 0x{:04X}, {} bytes",
                    attribute.conn_handle,
                    attribute.attr_handle,
                    attribute.data().len(),
                );

                // Check if this is a CCCD write (notification enable/disable)
                if is_cccd_handle(state.data_char_handle.0, attribute.attr_handle.0) {
                    let cccd = CccdValue::from_bytes(attribute.data());
                    state.notifications_enabled = cccd.notifications;
                    info!(
                        "CCCD updated: notifications={}, indications={}",
                        cccd.notifications, cccd.indications
                    );

                    if state.notifications_enabled {
                        info!("Notifications ENABLED - will send updates");
                    } else {
                        info!("Notifications DISABLED");
                    }
                }
                // Check if this is a characteristic value write
                else if is_value_handle(state.data_char_handle.0, attribute.attr_handle.0) {
                    match Payload::decode(attribute.data()) {
                        Ok(payload) => {
                            info!("Payload received: {}", payload);
                            PAYLOAD.signal(payload);
                        }
                        Err(e) => warn!(
                            "Payload rejected: {} (raw {:?})",
                            e,
                            attribute.data()
                        ),
                    }

                    // Echo the data back as a notification if enabled
                    if state.notifications_enabled {
                        if let Some(conn) = state.current_conn_handle {
                            // Increment counter and append to response
                            state.counter = state.counter.wrapping_add(1);
                            let mut response: heapless::Vec<u8, 33> = heapless::Vec::new();
                            let _ = response.extend_from_slice(attribute.data());
                            let _ = response.push(state.counter);

                            match gatt.notify(
                                conn,
                                state.service_handle,
                                state.data_char_handle,
                                &response,
                            ) {
                                Ok(()) => {
                                    info!("Notification sent: {} bytes", response.len());
                                }
                                Err(e) => {
                                    error!("Failed to send notification: {:?}", e);
                                }
                            }
                        }
                    }
                }
            }

            Event::Vendor(VendorEvent::GattNotificationComplete(attr_handle)) => {
                info!("Notification complete: conn attr 0x{:04X}", attr_handle);
            }

            Event::Vendor(VendorEvent::AttExchangeMtuResponse(AttExchangeMtuResponse {
                conn_handle,
                server_rx_mtu,
            })) => {
                info!(
                    "MTU exchanged: conn 0x{:04X}, MTU={}",
                    conn_handle.0, server_rx_mtu
                );
                // Update connection MTU
                if let Some(conn) = ble.get_connection_mut(*conn_handle) {
                    conn.update_mtu(*server_rx_mtu as u16);
                }
            }

            _ => {
                // Log other events at debug level
                debug!("Event: {:?}", event);
            }
        }
    }
}
