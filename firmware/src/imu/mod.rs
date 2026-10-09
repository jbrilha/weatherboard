mod calibration;
pub mod drivers;
pub mod fusion;

use defmt::{Format, error, info, warn};
use embassy_stm32::{
    Peri,
    exti::ExtiInput,
    gpio::{Level, OutputOpenDrain, Pin, Pull, Speed},
    i2c,
    time::khz,
};
use embassy_time::{Duration, Instant, block_for};

pub use crate::board::ImuPeripherals;
use crate::board::Irqs;
use crate::imu::drivers::{ImuDriver, ImuDriverError};
use crate::imu::fusion::{Attitude, Fusion};

pub use mpu6500::{AccelFs, GyroFs, Sample, SelfTestResult, Vec3};
/// sample with bias removed and scaled to irl units
#[derive(Debug, Clone, Copy, Format)]
pub struct Reading {
    /// g
    pub accel: Vec3<f32>,
    /// deg/s
    pub gyro: Vec3<f32>,
    /// deg C
    pub temp: f32,
}

#[derive(Debug)]
pub enum ImuError {
    Driver(ImuDriverError),
    SelfTest(SelfTestResult),
}

impl From<ImuDriverError> for ImuError {
    fn from(err: ImuDriverError) -> Self {
        ImuError::Driver(err)
    }
}

pub struct Imu {
    pub driver: ImuDriver,
    gyro_bias: Vec3<i32>,
    accel_bias: Vec3<i32>,
    fusion: Fusion,
    last_update: Option<Instant>,
}

impl Imu {
    pub fn new(mut ip: ImuPeripherals) -> Self {
        recover_bus(ip.scl.reborrow(), ip.sda.reborrow());

        let mut config = i2c::Config::default();
        config.frequency = khz(400);

        let i2c = i2c::I2c::new(ip.i2c, ip.scl, ip.sda, ip.dma_tx, ip.dma_rx, Irqs, config);

        let irq = ExtiInput::new(ip.irq_pin, ip.irq, Pull::Down, Irqs);

        Self {
            driver: drivers::new(i2c, irq),
            gyro_bias: Vec3::default(),
            accel_bias: Vec3::default(),
            fusion: Fusion::new(),
            last_update: None,
        }
    }

    pub async fn self_test(&mut self) -> Result<SelfTestResult, ImuError> {
        let result = self.driver.self_test().await?;
        if result.passed {
            Ok(result)
        } else {
            Err(ImuError::SelfTest(result))
        }
    }

    pub async fn init(&mut self) -> Result<(), ImuDriverError> {
        self.driver.init().await
    }

    pub async fn wait_for_data(&mut self) {
        let _ = self.driver.wait_for_isr().await;
    }

    pub async fn read_raw(&mut self) -> Result<Sample, ImuDriverError> {
        self.driver.read_all().await
    }

    pub async fn read(&mut self) -> Result<Reading, ImuDriverError> {
        let raw = self.driver.read_all().await?;
        Ok(self.convert(raw))
    }

    fn convert(&self, raw: Sample) -> Reading {
        Reading {
            accel: raw
                .accel
                .widen()
                .sub(self.accel_bias)
                .scale(self.driver.accel_fs().lsb_per_g()),
            gyro: raw
                .gyro
                .widen()
                .sub(self.gyro_bias)
                .scale(self.driver.gyro_fs().lsb_per_dps()),
            temp: raw.temp as f32 / TEMP_SENSITIVITY + TEMP_OFFSET_C,
        }
    }

    /* pub async fn calibrate(&mut self, samples: u16) -> Result<(), D::DriverError> {
        let mut gyro = Vec3::<i32>::default();
        let mut accel = Vec3::<i32>::default();

        for _ in 0..samples {
            self.driver.wait_for_isr().await;
            let raw = self.driver.read_all().await?;
            gyro = gyro.zip(raw.gyro.widen(), |a, b| a + b);
            accel = accel.zip(raw.accel.widen(), |a, b| a + b);
        }

        let n = samples as i32;
        self.gyro_bias = gyro.map(|v| v / n);

        // this was making things drift
        // let one_g = self.driver.accel_fs().lsb_per_g() as i32;
        // self.accel_bias = accel.map(|v| v / n).sub(Vec3::new(0, 0, one_g));

        info!(
            "imu bias gyro: {} | accel: {}",
            self.gyro_bias, self.accel_bias
        );
        Ok(())
    }

    pub async fn calibrate_accel_screen_down(
        &mut self,
        samples: u16,
    ) -> Result<bool, D::DriverError> {
        let mut accel = Vec3::<i32>::default();

        for _ in 0..samples {
            self.driver.wait_for_isr().await;
            let raw = self.driver.read_all().await?;
            accel = accel.zip(raw.accel.widen(), |a, b| a + b);
        }

        let average = accel.map(|v| v / samples as i32);
        let one_g = self.driver.accel_fs().lsb_per_g() as i32;

        if average.z.abs() < one_g * 9 / 10
            || average.x.abs() > one_g / 4
            || average.y.abs() > one_g / 4
        {
            error!(
                "board is not screen down, keeping accel bias: {} {} {}",
                average.x, average.y, average.z
            );
            return Ok(false);
        }

        let up = if average.z > 0 { one_g } else { -one_g };
        self.accel_bias = average.sub(Vec3::new(0, 0, up));

        info!(
            "ACCEL_BIAS: Vec3::new({}, {}, {}),",
            self.accel_bias.x, self.accel_bias.y, self.accel_bias.z
        );
        info!(
            "screen down reference {} {} {}, 1g = {}",
            average.x, average.y, average.z, one_g
        );

        Ok(true)
    } */

    pub async fn track(&mut self) -> Result<Attitude, ImuDriverError> {
        let reading = self.read().await?;

        let now = Instant::now();
        let dt = match self.last_update {
            Some(last) => (now - last).as_micros() as f32 / 1_000_000.0,
            None => 0.0,
        };
        self.last_update = Some(now);

        Ok(self.fusion.update(
            to_screen_frame(reading.accel),
            to_screen_frame(reading.gyro),
            dt,
        ))
    }

    pub fn attitude(&self) -> Attitude {
        self.fusion.attitude()
    }
}

fn recover_bus(scl: Peri<'_, impl Pin>, sda: Peri<'_, impl Pin>) {
    let half_period = Duration::from_micros(5);
    let mut scl = OutputOpenDrain::new_pull(scl, Level::High, Speed::Low, Pull::Up);
    let mut sda = OutputOpenDrain::new_pull(sda, Level::High, Speed::Low, Pull::Up);
    block_for(half_period);

    if sda.is_high() {
        return;
    }

    warn!("i2c bus stuck, clocking it free");
    for _ in 0..9 {
        scl.set_low();
        block_for(half_period);
        scl.set_high();
        block_for(half_period);
        if sda.is_high() {
            break;
        }
    }

    scl.set_low();
    block_for(half_period);
    sda.set_low();
    block_for(half_period);
    scl.set_high();
    block_for(half_period);
    sda.set_high();
    block_for(half_period);
}

fn to_screen_frame(v: Vec3<f32>) -> Vec3<f32> {
    Vec3::new(v.z, v.x, v.y)
}

const TEMP_SENSITIVITY: f32 = 333.87; // LSB/degC, datasheet §3.4
const TEMP_OFFSET_C: f32 = 21.0;

const ACCEL_BIAS: Vec3<i32> = Vec3::new(989, 253, -1142);

const CALIBRATION_SAMPLES: u16 = 500;
const LOG_EVERY: u32 = 2000;

#[embassy_executor::task]
pub async fn imu_task(mut imu: Imu) {
    if imu.init().await.is_err() {
        error!("failed to init imu");
        return;
    }
    info!("imu ready");

    match imu.self_test().await {
        Ok(res) => info!(
            "self test okay gyro: {} | accel: {} | offset: {}",
            res.gyro, res.accel, res.offset
        ),
        Err(ImuError::SelfTest(res)) => error!(
            "self test error gyro: {} | accel: {} | offset: {}",
            res.gyro, res.accel, res.offset
        ),
        Err(ImuError::Driver(_)) => error!("driver error in IMU self test"),
    }

    if imu.calibrate(CALIBRATION_SAMPLES).await.is_err() {
        error!("failed to calibrate imu");
        return;
    }

    let (gyro_bias, _) = imu.bias();
    imu.set_bias(gyro_bias, ACCEL_BIAS);
    info!(
        "accel bias {} {} {}",
        ACCEL_BIAS.x, ACCEL_BIAS.y, ACCEL_BIAS.z
    );

    let mut n = 0;
    loop {
        imu.wait_for_data().await;
        match imu.track().await {
            Ok(attitude) => {
                if n % LOG_EVERY == 0 {
                    info!(
                        "roll: {} | pitch: {} | yaw: {}",
                        attitude.roll, attitude.pitch, attitude.yaw
                    );
                }
            }
            Err(_) => error!("failed to read imu"),
        }
        n = n.wrapping_add(1);
    }
}
