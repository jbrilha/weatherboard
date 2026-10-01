mod ili9341;
mod ili9341_cmd;
pub mod rgb565;

use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;
use embassy_stm32::{gpio::Output, spi::Spi};
use embassy_stm32::{mode::Async, spi::mode::Master};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::Delay;
use embedded_hal_bus::spi::ExclusiveDevice;
pub use ili9341::{HEIGHT, ILI9341, WIDTH};

use crate::spi_utils::{SpiDev, SpiDevError};

pub type LcdDriver = ILI9341<SpiDev>;
pub type LcdDriverError = SpiDevError;

pub fn new(spi: SpiDev, dc: Output<'static>, reset: Output<'static>) -> LcdDriver {
    ILI9341::new_spi(spi, dc, reset)
}
