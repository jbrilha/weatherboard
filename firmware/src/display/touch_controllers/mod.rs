pub mod xpt2046;

use defmt::info;
use embassy_stm32::{exti::ExtiInput, mode::Async};

use crate::{display::touch_controllers::xpt2046::XPT2046, spi_utils::SpiDev};

pub type TouchController = xpt2046::XPT2046<SpiDev>;

pub fn new(spi: SpiDev, isr_pin: ExtiInput<'static, Async>) -> TouchController {
    XPT2046::new_spi(spi, isr_pin)
}

#[embassy_executor::task]
pub async fn touch_isr_task(mut touch: TouchController) {
    loop {
        touch.wait_touched().await;
        info!("touched!");
    }
}

pub struct Calibration {
    pub x1: i32,
    pub x2: i32,
    pub y1: i32,
    pub y2: i32,
    pub sx: i32,
    pub sy: i32,
    pub swap_xy: bool,
}

// x1/y1 are the raw values at screen 0, x2/y2 the raw values at sx/sy
// from touch_calibration_task
//
//   target        screen      raw
//   top left      ( 30,  40)  ( 650,  541)
//   top right     (210,  40)  (3364,  530)
//   bottom left   ( 30, 280)  ( 654, 3210)
//   bottom right  (210, 280)  (3380, 3242)
pub const CALIBRATION: Calibration = Calibration {
    x1: 199,
    x2: 3825,
    y1: 87,
    y2: 3674,
    sx: 240,
    sy: 320,
    swap_xy: false,
};
