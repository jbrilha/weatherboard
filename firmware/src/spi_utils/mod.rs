use core::convert::Infallible;

use embassy_stm32::{
    Peri,
    gpio::Output,
    mode::Async,
    spi::{MisoPin, MosiPin, SckPin, Spi, mode::Master},
};
use embassy_time::Delay;
use embedded_hal_bus::spi::{DeviceError, ExclusiveDevice};

pub type SpiDev = ExclusiveDevice<Spi<'static, Async, Master>, Output<'static>, Delay>;
pub type SpiDevError = DeviceError<embassy_stm32::spi::Error, Infallible>;
