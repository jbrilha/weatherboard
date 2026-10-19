#![no_std]

pub mod ipc;
pub mod weather;

use serde::{Deserialize, Serialize};
pub use uuid::Uuid;

use crate::weather::WeatherPayload;

pub const PAYLOAD_VERSION: u8 = u8::MAX;
pub const MAX_PAYLOAD_LEN: usize = 512;
pub const TEST_LEN: usize = 32;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ConfigPayload {
    byte: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ToBoard {
    Weather(WeatherPayload),
    Config(ConfigPayload),
}

pub trait Message: Serialize + Sized {
    fn encode<'a>(&self, buf: &'a mut [u8]) -> Result<&'a [u8], postcard::Error> {
        postcard::to_slice(self, buf).map(|bytes| &*bytes)
    }

    fn decode<'a>(bytes: &'a [u8]) -> Result<Self, DecodeError>
    where
        Self: Deserialize<'a>,
    {
        postcard::from_bytes(bytes).map_err(|_| DecodeError::Malformed)
    }
}

impl Message for ToBoard {}

#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DecodeError {
    Malformed,
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
