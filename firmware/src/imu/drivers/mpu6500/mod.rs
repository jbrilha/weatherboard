pub mod registers;
mod self_test;

use defmt::{info, warn};
use embassy_stm32::{
    exti::ExtiInput,
    gpio::Output,
    mode::Async,
    spi::{Spi, mode::Master},
};
use embassy_time::{Delay, Timer};
use embedded_hal::spi::Operation;
use embedded_hal_async::spi::SpiDevice;
use embedded_hal_bus::spi::ExclusiveDevice;

use crate::imu::drivers::Driver;
use crate::imu::drivers::mpu6500::registers::Register;
use crate::imu::{AccelFs, GyroFs, Sample, SelfTestResult, Vec3};

pub enum Interrupt {
    MotionDetection,
    FIFOOverflow,
    DataReady,
}

impl Interrupt {
    const fn mask(self) -> u8 {
        match self {
            Interrupt::MotionDetection => 1 << 6,
            Interrupt::FIFOOverflow => 1 << 4,
            Interrupt::DataReady => 1 << 0,
        }
    }
}

#[derive(Debug)]
pub enum MPU6500Error<E> {
    Spi(E),
    NoDevice(u8),
}

impl<E> From<E> for MPU6500Error<E> {
    fn from(e: E) -> Self {
        MPU6500Error::Spi(e)
    }
}

const WHO_AM_I: u8 = 0x70;
const DLPF_41HZ: u8 = 0b011;
const FS_MASK: u8 = 0b11000;

pub struct MPU6500<S: SpiDevice> {
    dev: S,
    isr_pin: ExtiInput<'static, Async>,
    gyro_fs: GyroFs,
    accel_fs: AccelFs,
}

impl<S: SpiDevice> MPU6500<S> {
    pub fn new(dev: S, isr_pin: ExtiInput<'static, Async>) -> Self {
        Self {
            dev,
            isr_pin,
            gyro_fs: GyroFs::Dps250,
            accel_fs: AccelFs::G2,
        }
    }

    pub(crate) async fn write_reg(
        &mut self,
        reg: Register,
        value: u8,
    ) -> Result<(), MPU6500Error<S::Error>> {
        self.dev
            .transaction(&mut [Operation::Write(&[reg.addr(), value])])
            .await?;
        Ok(())
    }

    pub(crate) async fn read_regs<const N: usize>(
        &mut self,
        reg: Register,
    ) -> Result<[u8; N], MPU6500Error<S::Error>> {
        let mut buf = [0u8; N];
        self.dev
            .transaction(&mut [
                Operation::Write(&[reg.read_addr()]),
                Operation::Read(&mut buf),
            ])
            .await?;
        Ok(buf)
    }

    pub(crate) async fn read_reg(&mut self, reg: Register) -> Result<u8, MPU6500Error<S::Error>> {
        Ok(self.read_regs::<1>(reg).await?[0])
    }

    async fn modify_reg(
        &mut self,
        reg: Register,
        mask: u8,
        value: u8,
    ) -> Result<(), MPU6500Error<S::Error>> {
        let current = self.read_reg(reg).await?;
        self.write_reg(reg, (current & !mask) | (value & mask))
            .await
    }

    pub async fn enable_interrupt(
        &mut self,
        interrupt: Interrupt,
    ) -> Result<(), MPU6500Error<S::Error>> {
        let mask = interrupt.mask();
        self.modify_reg(Register::IntEnable, mask, mask).await
    }

    pub async fn disable_interrupt(
        &mut self,
        interrupt: Interrupt,
    ) -> Result<(), MPU6500Error<S::Error>> {
        let mask = interrupt.mask();
        self.modify_reg(Register::IntEnable, mask, 0).await
    }
}

impl<S: SpiDevice> Driver for MPU6500<S> {
    type DriverError = MPU6500Error<S::Error>;

    async fn init(&mut self) -> Result<(), Self::DriverError> {
        // reset then wait for the internal registers to settle
        self.write_reg(Register::PwrMgmt1, 0b10000000).await?;
        Timer::after_millis(100).await;

        // auto-select the best clock source, wake every axis
        self.write_reg(Register::PwrMgmt1, 0b001).await?;
        self.write_reg(Register::PwrMgmt2, 0b000000).await?;
        Timer::after_millis(10).await;

        // SPI only
        self.write_reg(Register::UserCtrl, 0b10000).await?;

        let id = self.read_reg(Register::WhoAmI).await?;
        if id == 0x00 || id == 0xFF {
            return Err(MPU6500Error::NoDevice(id));
        }
        if id != WHO_AM_I {
            warn!(
                "unexpected WHO_AM_I {=u8:#04x}, expected {=u8:#04x}",
                id, WHO_AM_I
            );
        }

        // 1kHz internal sample rate, no division
        self.write_reg(Register::SmplrtDiv, 0b00).await?;
        self.write_reg(Register::Config, DLPF_41HZ).await?;
        self.write_reg(Register::AccelConfig2, DLPF_41HZ).await?;

        let gyro_fs = self.gyro_fs;
        let accel_fs = self.accel_fs;
        self.set_gyro_fs(gyro_fs).await?;
        self.set_accel_fs(accel_fs).await?;

        // active high, push-pull
        self.write_reg(Register::IntPinCfg, 0b00000000).await?;
        self.enable_interrupt(Interrupt::DataReady).await?;

        info!("mpu6500 init done, id {=u8:#04x}", id);
        Ok(())
    }

    async fn read_gyro(&mut self) -> Result<Vec3<i16>, Self::DriverError> {
        let raw = self.read_regs::<6>(Register::GyroXOutH).await?;
        Ok(Vec3::from_be_bytes(raw))
    }

    async fn read_accel(&mut self) -> Result<Vec3<i16>, Self::DriverError> {
        let raw = self.read_regs::<6>(Register::AccelXOutH).await?;
        Ok(Vec3::from_be_bytes(raw))
    }

    async fn read_all(&mut self) -> Result<Sample, Self::DriverError> {
        // ACCEL_XOUT_H through GYRO_ZOUT_L are contiguous, temperature included
        let raw = self.read_regs::<14>(Register::AccelXOutH).await?;
        Ok(Sample {
            accel: Vec3::from_be_bytes([raw[0], raw[1], raw[2], raw[3], raw[4], raw[5]]),
            temp: i16::from_be_bytes([raw[6], raw[7]]),
            gyro: Vec3::from_be_bytes([raw[8], raw[9], raw[10], raw[11], raw[12], raw[13]]),
        })
    }

    async fn self_test(&mut self) -> Result<SelfTestResult, Self::DriverError> {
        self.run_self_test().await
    }

    async fn set_sleep(&mut self) -> Result<(), Self::DriverError> {
        self.write_reg(Register::PwrMgmt1, 0b01000000).await?;
        Ok(())
    }

    async fn set_gyro_standby(&mut self) -> Result<(), Self::DriverError> {
        self.write_reg(Register::PwrMgmt1, 0b10000).await?;
        Ok(())
    }

    async fn set_gyro_fs(&mut self, fs: GyroFs) -> Result<(), Self::DriverError> {
        self.modify_reg(Register::GyroConfig, FS_MASK, fs.bits())
            .await?;
        self.gyro_fs = fs;
        Ok(())
    }

    async fn set_accel_fs(&mut self, fs: AccelFs) -> Result<(), Self::DriverError> {
        self.modify_reg(Register::AccelConfig, FS_MASK, fs.bits())
            .await?;
        self.accel_fs = fs;
        Ok(())
    }

    fn gyro_fs(&self) -> GyroFs {
        self.gyro_fs
    }

    fn accel_fs(&self) -> AccelFs {
        self.accel_fs
    }

    async fn wait_for_isr(&mut self) {
        self.isr_pin.wait_for_rising_edge().await;
    }
}

impl MPU6500<ExclusiveDevice<Spi<'static, Async, Master>, Output<'static>, Delay>> {
    pub fn new_spi(
        dev: ExclusiveDevice<Spi<'static, Async, Master>, Output<'static>, Delay>,
        isr_pin: ExtiInput<'static, Async>,
    ) -> Self {
        Self::new(dev, isr_pin)
    }
}
