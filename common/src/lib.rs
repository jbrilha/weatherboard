#![no_std]

pub mod ipc;

use serde::{Deserialize, Serialize};
pub use uuid::Uuid;

pub const PAYLOAD_VERSION: u8 = u8::MAX;
pub const MAX_PAYLOAD_LEN: usize = 512;
pub const TEST_LEN: usize = 32;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Payload {
    pub version: u8,
    pub counter: u32,
    pub fluff: [u64; TEST_LEN],
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DecodeError {
    Malformed,
    UnsupportedVersion(u8),
}

impl Payload {
    pub fn new(counter: u32) -> Self {
        let buf = [0u8; TEST_LEN];
        let mut i = 0;
        let buf = buf.map(|_| {
            i = i + 1;
            // u64::MAX - i
            i
        });
        Self {
            version: u8::MAX,
            counter: u32::MAX,
            fluff: buf,
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
