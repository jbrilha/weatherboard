#![no_std]

pub use uuid::Uuid;

fn namespace() -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_URL, b"weatherboard")
}

pub fn display_service_uuid() -> Uuid {
    Uuid::new_v5(&namespace(), b"display-service")
}

pub fn weather_payload_uuid() -> Uuid {
    Uuid::new_v5(&namespace(), b"weather-payload")
}
