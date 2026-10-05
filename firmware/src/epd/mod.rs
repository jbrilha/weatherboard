use common::Payload;
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
    geometry::{Point, Size},
    mono_font::{MonoTextStyle, iso_8859_1::FONT_10X20},
    primitives::{Circle, Line, Primitive, PrimitiveStyle, Rectangle},
    text::Text,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use heapless::String;
use weact_studio_epd::{TriColor, WeActStudio420TriColorDriver, graphics::Display420TriColor};

use crate::ble::PAYLOAD;
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
            refresh: Refresh::FastFull,
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

#[embassy_executor::task]
pub async fn epd_ble_task(mut epd: Epd) {
    if epd.init().await.is_err() {
        error!("failed to init epd");
        return;
    }
    info!("epd ready");
    let style = MonoTextStyle::new(&FONT_10X20, TriColor::Black);
    let mut current: Option<Payload> = None;
    loop {
        let mut counter: String<16> = String::new();
        let mut version: String<16> = String::new();
        match current {
            Some(payload) => {
                write!(counter, "counter: {}", payload.counter).unwrap();
                write!(version, "version: {}", payload.version).unwrap();
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

        if let Either::First(payload) = select(PAYLOAD.wait(), Timer::after_secs(15 * 60)).await {
            current = Some(payload);
        }
        epd.wake_up().await.unwrap();
    }
}

fn draw_test_pattern(display: &mut Display420TriColor, refresh: Refresh, round: u32) {
    display.clear(TriColor::White);

    let (primary, secondary) = if round % 2 == 0 {
        (TriColor::Black, TriColor::Red)
    } else {
        (TriColor::Red, TriColor::Black)
    };

    let label = match refresh {
        Refresh::Full => "full refresh",
        Refresh::FastFull => "fast full refresh",
    };
    Text::new(
        label,
        Point::new(10, 24),
        MonoTextStyle::new(&FONT_10X20, primary),
    )
    .draw(display)
    .unwrap();
    Text::new(
        "black red white",
        Point::new(10, 48),
        MonoTextStyle::new(&FONT_10X20, secondary),
    )
    .draw(display)
    .unwrap();

    let swatches = [TriColor::Black, TriColor::Red, TriColor::White];
    for (i, color) in swatches.into_iter().enumerate() {
        let top_left = Point::new(10 + i as i32 * 130, 64);
        Rectangle::new(top_left, Size::new(120, 90))
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)
            .unwrap();
        Rectangle::new(top_left, Size::new(120, 90))
            .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
            .draw(display)
            .unwrap();
    }

    for i in 0..40 {
        let x = 10 + i * 4;
        let color = if i % 2 == 0 { primary } else { secondary };
        Line::new(Point::new(x, 170), Point::new(x, 290))
            .into_styled(PrimitiveStyle::with_stroke(color, 1))
            .draw(display)
            .unwrap();
    }

    Circle::new(Point::new(190, 170), 120)
        .into_styled(PrimitiveStyle::with_fill(secondary))
        .draw(display)
        .unwrap();
    Circle::new(Point::new(220, 200), 60)
        .into_styled(PrimitiveStyle::with_fill(primary))
        .draw(display)
        .unwrap();

    Rectangle::new(Point::new(320, 170), Size::new(70, 120))
        .into_styled(PrimitiveStyle::with_stroke(secondary, 3))
        .draw(display)
        .unwrap();
}
