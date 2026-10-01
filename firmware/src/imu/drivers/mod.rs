use core::convert::Infallible;

use embassy_stm32::{exti::ExtiInput, i2c::I2c, i2c::Master, mode::Async};
use embassy_time::Delay;
use mpu6500::{ADDRESS_AD0_LOW, I2cInterface, MPU6500, MPU6500Error};

pub type ImuBus = I2c<'static, Async, Master>;
pub type ImuDriver = MPU6500<I2cInterface<ImuBus>, ExtiInput<'static, Async>, Delay>;
pub type ImuDriverError = MPU6500Error<embassy_stm32::i2c::Error, Infallible>;

pub fn new(i2c: ImuBus, isr_pin: ExtiInput<'static, Async>) -> ImuDriver {
    MPU6500::new_i2c(i2c, ADDRESS_AD0_LOW, isr_pin, Delay)
}
