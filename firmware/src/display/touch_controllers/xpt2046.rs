use defmt::{debug, info};
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

use crate::display::touch_controllers::CALIBRATION;

const CMD_Y: u8 = 0x90;
const CMD_Z1: u8 = 0xB0;
const CMD_Z2: u8 = 0xC0;
const CMD_X: u8 = 0xD0;

// highest resistance that still counts as a touch (real contact ~8000, touch/lift junk ~18000)
const R_THRESHOLD: i32 = 15_000;

const READ_SAMPLES: usize = 16;
const READ_MIN_SAMPLES: usize = 4;
const READ_SETTLE_MS: u64 = 20;

pub struct RawSample {
    pub x: i32,
    pub y: i32,
    pub resistance: i32,
}

// fig 12: there's an acquire clock before D11, so 16 clocks yields 1 junk bit + D11..D0 + 3 zeros
fn to_12bit(bytes: [u8; 2]) -> i32 {
    ((u16::from_be_bytes(bytes) >> 3) & 0x0FFF) as i32
}

pub struct XPT2046<S: SpiDevice> {
    dev: S,
    isr_pin: ExtiInput<'static, Async>,
}

// https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/spi_display.rs
impl<S: SpiDevice> XPT2046<S> {
    pub fn new(dev: S, isr_pin: ExtiInput<'static, Async>) -> Self {
        Self { dev, isr_pin }
    }

    pub async fn wait_touched(&mut self) {
        self.isr_pin.wait_for_falling_edge().await;
    }

    pub fn is_touched(&self) -> bool {
        self.isr_pin.is_low()
    }

    pub async fn read_raw(&mut self) -> Result<RawSample, S::Error> {
        let mut z1 = [0u8; 2];
        let mut z2 = [0u8; 2];
        let mut raw_row = [0u8; 2];
        let mut raw_col = [0u8; 2];

        self.dev
            .transaction(&mut [
                Operation::Write(&[CMD_Z1]),
                Operation::Read(&mut z1),
                Operation::Write(&[CMD_Z2]),
                Operation::Read(&mut z2),
                Operation::Write(&[CMD_Y]),
                Operation::Read(&mut raw_row),
                Operation::Write(&[CMD_X]),
                Operation::Read(&mut raw_col),
            ])
            .await?;

        let z1 = to_12bit(z1);
        let z2 = to_12bit(z2);
        let raw_row = to_12bit(raw_row);
        let raw_col = to_12bit(raw_col);

        // equation (3) p20: R = R_X-Plate * (X / 4096) * (Z2/Z1 - 1)
        // R_X-Plate and the /4096 are constant so they just fold into R_THRESHOLD,
        // and (Z2/Z1 - 1) -> (Z2 - Z1)/Z1 to stay in integer land
        let resistance = if z1 == 0 {
            i32::MAX
        } else {
            raw_col * (z2 - z1) / z1
        };

        Ok(RawSample {
            x: raw_col,
            y: raw_row,
            resistance,
        })
    }

    pub async fn read(&mut self) -> Option<(i32, i32)> {
        // PENIRQ fires before the divider settles, so reading right away was giving me junk that
        // clamped to a screen edge
        Timer::after_millis(READ_SETTLE_MS).await;
        self.track().await
    }

    pub async fn track(&mut self) -> Option<(i32, i32)> {
        let mut xs = [0i32; READ_SAMPLES];
        let mut ys = [0i32; READ_SAMPLES];
        let mut n = 0;

        for _ in 0..READ_SAMPLES {
            if !self.is_touched() {
                break;
            }
            let sample = self.read_raw().await.ok()?;
            if sample.resistance > R_THRESHOLD {
                continue;
            }
            xs[n] = sample.x;
            ys[n] = sample.y;
            n += 1;
        }

        if n < READ_MIN_SAMPLES {
            return None;
        }

        xs[..n].sort_unstable();
        ys[..n].sort_unstable();
        let raw_x = xs[n / 2];
        let raw_y = ys[n / 2];

        debug!("raw x:{}, y:{}, samples:{}", raw_x, raw_y, n);

        let cal = &CALIBRATION;
        let (raw_x, raw_y) = if cal.swap_xy {
            (raw_y, raw_x)
        } else {
            (raw_x, raw_y)
        };

        let x = ((raw_x - cal.x1) * cal.sx / (cal.x2 - cal.x1)).clamp(0, cal.sx);
        let y = ((raw_y - cal.y1) * cal.sy / (cal.y2 - cal.y1)).clamp(0, cal.sy);
        Some((x, y))
    }
}

impl XPT2046<ExclusiveDevice<Spi<'static, Async, Master>, Output<'static>, Delay>> {
    pub fn new_spi(
        dev: ExclusiveDevice<Spi<'static, Async, Master>, Output<'static>, Delay>,
        isr_pin: ExtiInput<'static, Async>,
    ) -> Self {
        Self::new(dev, isr_pin)
    }
}
