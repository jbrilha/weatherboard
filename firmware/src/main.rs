#![no_std]
#![no_main]
mod board;
pub mod imu;
pub mod spi_utils;
pub mod state_machine;
mod tasks;
pub mod utils;

use defmt::info;
use embassy_executor::Spawner;

use crate::{
    board::Board,
    imu::{Imu, imu_task},
};

use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let b = Board::init();
    info!("init done");

    let imu = Imu::new(b.imu);
    spawner.spawn(imu_task(imu).unwrap());
}
