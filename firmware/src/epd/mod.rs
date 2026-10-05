use defmt::{Format, error, info};
use display_interface_spi::SPIInterface;
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{Level, Output, Pull, Speed},
    mode::Async,
    spi::{self, Spi},
    time::mhz,
};
use embassy_time::{Delay, Timer};
use embedded_graphics::{
    Drawable,
    geometry::{Point, Size},
    mono_font::{MonoTextStyle, iso_8859_1::FONT_10X20},
    primitives::{Circle, Line, Primitive, PrimitiveStyle, Rectangle},
    text::Text,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use weact_studio_epd::{TriColor, WeActStudio420TriColorDriver, graphics::Display420TriColor};

pub use crate::board::EpdPeripherals;
use crate::board::Irqs;
use crate::spi_utils::SpiDev;

pub type EpdDriver = WeActStudio420TriColorDriver<
    SPIInterface<SpiDev, Output<'static>>,
    ExtiInput<'static, Async>,
    Output<'static>,
    Delay,
>;

#[derive(Clone, Copy, Format)]
pub enum Refresh {
    Full,
    FastFull,
}

pub struct Epd {
    driver: EpdDriver,
    display: Display420TriColor,
    refresh: Refresh,
}

impl Epd {
    pub fn new(ep: EpdPeripherals) -> Self {
        let mut config = spi::Config::default();
        config.frequency = mhz(4);
        config.mode = spi::MODE_0;

        let spi = Spi::new_txonly(ep.spi, ep.sck, ep.mosi, ep.dma_tx, Irqs, config);

        let cs = Output::new(ep.cs, Level::High, Speed::Low);
        let dc = Output::new(ep.dc, Level::High, Speed::Low);
        let reset = Output::new(ep.reset, Level::High, Speed::Low);
        let busy = ExtiInput::new(ep.busy_pin, ep.busy, Pull::None, Irqs);

        let device = ExclusiveDevice::new(spi, cs, Delay).unwrap();

        let driver =
            WeActStudio420TriColorDriver::new(SPIInterface::new(device, dc), busy, reset, Delay);
        Epd {
            driver,
            display: Display420TriColor::new(),
            refresh: Refresh::Full,
        }
    }
    pub async fn init(&mut self) -> weact_studio_epd::Result<()> {
        self.driver.init().await
    }

    pub fn display(&mut self) -> &mut Display420TriColor {
        &mut self.display
    }

    pub fn set_refresh(&mut self, refresh: Refresh) {
        self.refresh = refresh;
    }

    pub async fn flush(&mut self) -> weact_studio_epd::Result<()> {
        match self.refresh {
            Refresh::Full => self.driver.full_update(&self.display).await,
            Refresh::FastFull => self.driver.fast_full_update(&self.display).await,
        }
    }

    pub async fn sleep(&mut self) -> weact_studio_epd::Result<()> {
        self.driver.sleep().await
    }

    pub async fn wake_up(&mut self) -> weact_studio_epd::Result<()> {
        self.driver.wake_up().await
    }
}

#[embassy_executor::task]
pub async fn epd_task(mut epd: Epd) {
    if epd.init().await.is_err() {
        error!("failed to init epd");
        return;
    }
    info!("epd ready");
    let style = MonoTextStyle::new(&FONT_10X20, TriColor::Black);
    loop {
        let d = epd.display();
        d.clear(TriColor::White);
        Text::new("12°C", Point::new(10, 30), style)
            .draw(d)
            .unwrap();
        epd.flush().await.unwrap();
        epd.sleep().await.unwrap();
        Timer::after_secs(15 * 60).await;
        epd.wake_up().await.unwrap();
    }
}

