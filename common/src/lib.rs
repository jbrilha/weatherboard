#![no_std]

use serde::{Deserialize, Serialize};
pub use uuid::Uuid;

pub const PAYLOAD_VERSION: u8 = 1;
pub const MAX_PAYLOAD_LEN: usize = 20;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Payload {
    pub version: u8,
    pub counter: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DecodeError {
    Malformed,
    UnsupportedVersion(u8),
}

impl Payload {
    pub fn new(counter: u32) -> Self {
        Self {
            version: PAYLOAD_VERSION,
            counter,
        }
    }

    pub fn encode<'a>(
        &self,
        buf: &'a mut [u8; MAX_PAYLOAD_LEN],
    ) -> Result<&'a [u8], postcard::Error> {
        postcard::to_slice(self, buf).map(|bytes| &*bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let payload: Self = postcard::from_bytes(bytes).map_err(|_| DecodeError::Malformed)?;
        if payload.version != PAYLOAD_VERSION {
            return Err(DecodeError::UnsupportedVersion(payload.version));
        }
        Ok(payload)
    }
}

fn namespace() -> Uuid {
    Uuid::new_v5(&Uuid::NAMESPACE_URL, b"weatherboard")
}

pub fn display_service_uuid() -> Uuid {
    Uuid::new_v5(&namespace(), b"display-service")
}

pub fn weather_payload_uuid() -> Uuid {
    Uuid::new_v5(&namespace(), b"weather-payload")
}
