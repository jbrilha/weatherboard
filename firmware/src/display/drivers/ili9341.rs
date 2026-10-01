use embassy_embedded_hal::shared_bus::asynch::spi::SpiDeviceWithConfig;
use embassy_stm32::{
    gpio::Output,
    mode::Async,
    spi::{Spi, mode::Master},
};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_time::{Delay, Timer};
use embedded_graphics::{
    pixelcolor::{Rgb565, raw::RawU16},
    prelude::*,
    primitives::Rectangle,
};
use embedded_hal_async::spi::SpiDevice;
use embedded_hal_bus::spi::ExclusiveDevice;
use static_cell::StaticCell;

use crate::display::{Orientation, Power, drivers::ili9341_cmd::Command};

const BUF_SIZE: usize = 512;
pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 320;

static FRAME_BUF: StaticCell<[u8; WIDTH * HEIGHT * 2]> = StaticCell::new();

pub struct ILI9341<S: SpiDevice> {
    dev: S,
    dc: Output<'static>,
    reset_pin: Output<'static>,
    frame_buf: &'static mut [u8; WIDTH * HEIGHT * 2],
    width: usize,
    height: usize,
}

impl<S: SpiDevice> ILI9341<S> {
    // https://github.com/embassy-rs/embassy/blob/main/examples/stm32f4/src/bin/lcd.rs
    // commands derived from https://github.com/STMicroelectronics/stm32-ili9341
    pub async fn init(&mut self) -> Result<(), S::Error> {
        self.hardware_reset().await;

        self.cmd_data(Command::PowerControlB, &[0x00, 0xC1, 0x30])
            .await?;
        self.cmd_data(Command::PowerOnSequenceControl, &[0x64, 0x03, 0x12, 0x81])
            .await?;
        self.cmd_data(Command::DriverTimingControlA, &[0x85, 0x00, 0x78])
            .await?;
        self.cmd_data(Command::PowerControlA, &[0x39, 0x2C, 0x00, 0x34, 0x02])
            .await?;
        self.cmd_data(Command::PumpRatioControl, &[0x20]).await?;
        self.cmd_data(Command::DriverTimingControlB, &[0x00, 0x00])
            .await?;
        self.cmd_data(Command::FrameRateControlNormalMode, &[0x00, 0x1B])
            .await?;
        self.cmd_data(Command::DisplayFunctionControl, &[0x0A, 0xA2])
            .await?;
        self.cmd_data(Command::PowerControl1, &[0x10]).await?;
        self.cmd_data(Command::PowerControl2, &[0x10]).await?;
        self.cmd_data(Command::VCOMControl1, &[0x45, 0x15]).await?;
        self.cmd_data(Command::VCOMControl2, &[0x90]).await?;
        self.cmd_data(Command::MemoryAccessControl, &[0x48]).await?;
        self.cmd_data(Command::COLMODPixelFormatSet, &[0x55])
            .await?;
        self.cmd_data(Command::Enable3Gamma, &[0x00]).await?;
        self.cmd_data(Command::DisplayFunctionControl, &[0x08, 0x82, 0x27])
            .await?;
        self.cmd(Command::MemoryWrite).await?;
        Timer::after_millis(200).await;
        self.cmd_data(Command::GammaSet, &[0x01]).await?;
        self.cmd_data(
            Command::PositiveGammaCorrection,
            &[
                0x0F, 0x31, 0x2B, 0x0C, 0x0E, 0x08, 0x4E, 0xF1, 0x37, 0x07, 0x10, 0x03, 0x0E, 0x09,
                0x00,
            ],
        )
        .await?;
        self.cmd_data(
            Command::NegativeGammaCorrection,
            &[
                0x00, 0x0E, 0x14, 0x03, 0x11, 0x07, 0x31, 0xC1, 0x48, 0x08, 0x0F, 0x0C, 0x31, 0x36,
                0x0F,
            ],
        )
        .await?;

        self.cmd(Command::DisplayInversionON).await?;
        self.cmd(Command::SleepOut).await?;
        Timer::after_millis(200).await;
        self.cmd(Command::DisplayON).await?;
        self.cmd(Command::MemoryWrite).await?;

        self.clear_screen(Rgb565::RED).await?;

        Ok(())
    }

    pub async fn flush(&mut self) -> Result<(), S::Error> {
        self.set_address_window(0, 0, (self.width - 1) as u16, (self.height - 1) as u16)
            .await?;
        self.cmd(Command::MemoryWrite).await?;
        self.dc.set_high();
        let mut local = [0u8; BUF_SIZE];
        let mut offset = 0;
        let total = self.width * self.height * 2;
        while offset < total {
            let end = (offset + BUF_SIZE).min(total);
            let len = end - offset;
            local[..len].copy_from_slice(&self.frame_buf[offset..end]);
            self.dev.write(&local[..len]).await?;
            offset = end;
        }
        Ok(())
    }

    pub async fn clear_screen(&mut self, color: Rgb565) -> Result<(), S::Error> {
        self.fill_rect(
            0,
            0,
            (self.width - 1) as u16,
            (self.height - 1) as u16,
            color,
        )
        .await
    }

    pub async fn set_orientation(&mut self, orientation: Orientation) -> Result<(), S::Error> {
        match orientation {
            Orientation::Portrait | Orientation::PortraitFlipped => {
                self.width = WIDTH;
                self.height = HEIGHT;
            }
            Orientation::Landscape | Orientation::LandscapeFlipped => {
                self.width = HEIGHT;
                self.height = WIDTH;
            }
        };
        self.cmd_data(Command::MemoryAccessControl, &[orientation.value()])
            .await?;
        self.set_address_window(0, 0, (self.width - 1) as u16, (self.height - 1) as u16)
            .await?;
        self.cmd(Command::MemoryWrite).await?;

        Ok(())
    }

    pub async fn power(&mut self, power: Power) -> Result<(), S::Error> {
        match power {
            Power::On => self.cmd(Command::DisplayON).await,
            Power::Off => self.cmd(Command::DisplayOFF).await,
        }
    }
}

impl ILI9341<ExclusiveDevice<Spi<'static, Async, Master>, Output<'static>, Delay>> {
    pub fn new_spi(
        dev: ExclusiveDevice<Spi<'static, Async, Master>, Output<'static>, Delay>,
        dc: Output<'static>,
        reset: Output<'static>,
    ) -> Self {
        Self::new(dev, dc, reset)
    }
}

impl<S: SpiDevice> ILI9341<S> {
    pub fn new(dev: S, dc: Output<'static>, reset_pin: Output<'static>) -> Self {
        let frame_buf = FRAME_BUF.init([0u8; WIDTH * HEIGHT * 2]);
        Self {
            dev,
            dc,
            reset_pin,
            frame_buf,
            width: WIDTH,
            height: HEIGHT,
        }
    }

    pub async fn hardware_reset(&mut self) {
        self.reset_pin.set_high();
        Timer::after_millis(5).await;
        self.reset_pin.set_low();
        Timer::after_millis(20).await;
        self.reset_pin.set_high();
        Timer::after_millis(120).await;
    }

    pub async fn software_reset(&mut self) -> Result<(), S::Error> {
        self.cmd(Command::SoftwareReset).await?;
        Timer::after_millis(5).await;
        Ok(())
    }

    pub async fn fill_rect(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
        color: Rgb565,
    ) -> Result<(), S::Error> {
        self.set_address_window(sx, sy, ex, ey).await?;
        self.cmd(Command::MemoryWrite).await?;

        let pixel_count = (ex as u32 - sx as u32 + 1) * (ey as u32 - sy as u32 + 1);
        let raw = RawU16::from(color).into_inner();
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;
        let mut buf = [0u8; BUF_SIZE];
        for i in (0..buf.len()).step_by(2) {
            buf[i] = hi;
            buf[i + 1] = lo;
        }

        let pixels_per_chunk = (BUF_SIZE / 2) as u32;
        let full_chunks = pixel_count / pixels_per_chunk;
        let remainder = (pixel_count % pixels_per_chunk) as usize * 2;

        for _ in 0..full_chunks {
            self.data(&buf).await?;
        }
        if remainder > 0 {
            self.data(&buf[..remainder]).await?;
        }
        Ok(())
    }

    async fn set_address_window(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), S::Error> {
        self.cmd_data(
            Command::ColumnAddressSet,
            &[(sx >> 8) as u8, sx as u8, (ex >> 8) as u8, ex as u8],
        )
        .await?;
        self.cmd_data(
            Command::PageAddressSet,
            &[(sy >> 8) as u8, sy as u8, (ey >> 8) as u8, ey as u8],
        )
        .await?;
        Ok(())
    }

    async fn cmd(&mut self, cmd: Command) -> Result<(), S::Error> {
        self.dc.set_low();
        self.dev.write(&[cmd.op_code()]).await
    }

    async fn data(&mut self, data: &[u8]) -> Result<(), S::Error> {
        self.dc.set_high();
        self.dev.write(data).await
    }

    async fn cmd_data(&mut self, cmd: Command, data: &[u8]) -> Result<(), S::Error> {
        self.cmd(cmd).await?;
        self.data(data).await
    }
}

impl<S: SpiDevice> OriginDimensions for ILI9341<S> {
    fn size(&self) -> Size {
        Size::new(self.width as u32, self.height as u32)
    }
}

impl<S: SpiDevice> DrawTarget for ILI9341<S> {
    type Color = Rgb565;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let width = self.width as i32;
        let height = self.height as i32;
        for Pixel(coord, color) in pixels {
            if coord.x >= 0 && coord.x < width && coord.y >= 0 && coord.y < height {
                let idx = (coord.y as usize * self.width + coord.x as usize) * 2;
                let raw = RawU16::from(color).into_inner();
                self.frame_buf[idx] = (raw >> 8) as u8;
                self.frame_buf[idx + 1] = raw as u8;
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        let clipped = area.intersection(&self.bounding_box());
        if clipped.size == Size::zero() {
            return Ok(());
        }

        let raw = RawU16::from(color).into_inner();
        let hi = (raw >> 8) as u8;
        let lo = raw as u8;
        let x0 = clipped.top_left.x as usize;
        let y0 = clipped.top_left.y as usize;
        let w = clipped.size.width as usize;
        let h = clipped.size.height as usize;

        for y in y0..y0 + h {
            let start = (y * self.width + x0) * 2;
            for px in self.frame_buf[start..start + w * 2].chunks_exact_mut(2) {
                px[0] = hi;
                px[1] = lo;
            }
        }
        Ok(())
    }
}
