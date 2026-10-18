use common::weather::WeatherPayload;
use core::fmt::Write;
use defmt::{Format, error, info};
use display_interface_spi::SPIInterface;
use embassy_futures::select::{Either, select};
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
    geometry::Point,
    mono_font::{MonoTextStyle, iso_8859_1::FONT_10X20},
    text::Text,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use heapless::String;
use ui::weather::WeatherInfo;
use weact_studio_epd::{TriColor, WeActStudio420TriColorDriver, graphics::Display420TriColor};

use crate::ble::WEATHER;
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
    pub fn new_with_refresh(ep: EpdPeripherals, refresh: Refresh) -> Self {
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
            refresh,
        }
    }

    pub fn new(ep: EpdPeripherals) -> Self {
        Epd::new_with_refresh(ep, Refresh::Full)
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

    let mut current: Option<WeatherPayload> = None;
    ui::weather::dashboard(&mut epd.display, &WeatherInfo::default()).unwrap();

    loop {
        match current {
            Some(payload) => {
                let info = WeatherInfo::from(&payload);
                ui::weather::dashboard(&mut epd.display, &info).unwrap();
            }
            None => {}
        }

        epd.flush().await.unwrap();
        epd.sleep().await.unwrap();

        if let Either::First(payload) = select(WEATHER.wait(), Timer::after_secs(15 * 60)).await {
            current = Some(payload);
        }

        epd.wake_up().await.unwrap();
    }
}

#[embassy_executor::task]
pub async fn epd_ble_task(mut epd: Epd) {
    if epd.init().await.is_err() {
        error!("failed to init epd");
        return;
    }
    info!("epd ready");
    let style = MonoTextStyle::new(&FONT_10X20, TriColor::Black);
    let mut current: Option<WeatherPayload> = None;
    loop {
        let mut counter: String<16> = String::new();
        let mut version: String<16> = String::new();
        match current {
            Some(payload) => {
                write!(counter, "counter: {}", payload.temp_c).unwrap();
                write!(version, "version: {}", payload.uvi_tenths).unwrap();
            }
            None => {
                write!(counter, "waiting for hub").unwrap();
                write!(version, "waiting for hub").unwrap();
            }
        }

        let d = epd.display();
        d.clear(TriColor::White);
        Text::new(&counter, Point::new(10, 30), style)
            .draw(d)
            .unwrap();
        Text::new(&version, Point::new(10, 50), style)
            .draw(d)
            .unwrap();
        epd.flush().await.unwrap();
        epd.sleep().await.unwrap();

        if let Either::First(payload) = select(WEATHER.wait(), Timer::after_secs(15 * 60)).await {
            current = Some(payload);
        }
        epd.wake_up().await.unwrap();
    }
}

fn draw_test_pattern(display: &mut Display420TriColor, refresh: Refresh, round: u32) {
    let label = match refresh {
        Refresh::Full => "full refresh",
        Refresh::FastFull => "fast full refresh",
    };
    ui::test_pattern(display, label, round).unwrap();
}

fn draw_weather_dashboaard(display: &mut Display420TriColor) {
    ui::weather::dashboard(display, &WeatherInfo::default()).unwrap();
}
