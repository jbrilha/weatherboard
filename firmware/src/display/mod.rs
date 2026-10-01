pub mod drivers;
pub mod touch_controllers;

use crate::board::Irqs;
pub use crate::board::{DisplayPeripherals, TouchPeripherals};
use crate::display::drivers::{HEIGHT, LcdDriver, LcdDriverError, WIDTH};
use crate::display::requests::{DisplayReply, DisplayRequest};
use crate::display::touch_controllers::TouchController;
use crate::spi_utils::SpiDevError;
use crate::state_machine::{Node, StateError, StateMachine, Transition};
use crate::storage::file_system::{self as FS, FsError, LS_ENTRIES};
use defmt::{error, info};
use embassy_stm32::{
    exti::ExtiInput,
    gpio::{Level, Output, Pull, Speed},
    spi::{self, Spi},
    time::{khz, mhz},
};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Delay, Timer};
use embedded_graphics::Pixel;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_7X14;
use embedded_graphics::prelude::{Primitive, WebColors};
use embedded_graphics::primitives::StyledDrawable;
use embedded_graphics::text::Text;
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Point, Size},
    pixelcolor::{Rgb565, RgbColor},
    primitives::{Line, PrimitiveStyleBuilder, Rectangle},
};
use embedded_hal_bus::spi::ExclusiveDevice;
use heapless::Vec;

static DISP_REQ: Channel<CriticalSectionRawMutex, DisplayRequest, 1> = Channel::new();
static DISP_REP: Channel<CriticalSectionRawMutex, Result<DisplayReply, DisplayError>, 1> =
    Channel::new();

#[derive(Debug)]
pub enum DisplayError {
    StateMachine(StateError),
    Spi(SpiDevError),
    Driver(LcdDriverError),
    Fs(FsError),
}

impl From<FsError> for DisplayError {
    fn from(err: FsError) -> Self {
        DisplayError::Fs(err)
    }
}

impl From<StateError> for DisplayError {
    fn from(err: StateError) -> Self {
        DisplayError::StateMachine(err)
    }
}

impl From<SpiDevError> for DisplayError {
    fn from(err: SpiDevError) -> Self {
        DisplayError::Spi(err)
    }
}

pub struct Display {
    pub driver: LcdDriver,
    pub touch: TouchController,
    backlight_pin: Output<'static>,
    cursor: usize,
    scroll_offset: usize,
    orientation: Orientation,
    switch: bool,
    // statem: StateMachine<DisplayState, DisplayError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Orientation {
    Portrait = 0x48,
    Landscape = 0x28,
    PortraitFlipped = 0x88,
    LandscapeFlipped = 0xE8,
}

impl Orientation {
    pub const fn value(self) -> u8 {
        self as u8
    }

    pub const fn as_str<'a>(self) -> &'a str {
        match self {
            Orientation::Portrait => "Portrait",
            Orientation::Landscape => "Landscape",
            Orientation::PortraitFlipped => "PortraitFlipped",
            Orientation::LandscapeFlipped => "LandscapeFlipped",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Orientation::Portrait => Orientation::Landscape,
            Orientation::Landscape => Orientation::PortraitFlipped,
            Orientation::PortraitFlipped => Orientation::LandscapeFlipped,
            Orientation::LandscapeFlipped => Orientation::Portrait,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Backlight {
    On,
    Off,
    Toggle,
}

#[derive(Debug, Clone, Copy)]
pub enum Power {
    On,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisplayState {
    Browsing,
    Playing,
}

impl Display {
    fn new_statem() -> Result<StateMachine<DisplayState, DisplayError>, StateError> {
        // let to_playing = Transition::new(condition, pre, next, post);
        let transitions: Vec<Transition<DisplayState, DisplayError>, 5> = Vec::new();
        let initial = Node::new(DisplayState::Browsing, transitions);
        StateMachine::new(initial)
    }

    pub fn new(dp: DisplayPeripherals, tp: TouchPeripherals) -> Self {
        Self::new_with_orientation(dp, tp, Orientation::Portrait)
    }

    pub fn new_with_orientation(
        dp: DisplayPeripherals,
        tp: TouchPeripherals,
        orientation: Orientation,
    ) -> Self {
        let mut lcd_config = spi::Config::default();
        lcd_config.frequency = mhz(40);
        lcd_config.mode = spi::MODE_0;

        let lcd_spi = Spi::new(
            dp.spi, dp.sck, dp.mosi, dp.miso, dp.dma_tx, dp.dma_rx, Irqs, lcd_config,
        );

        let lcd_dc = Output::new(dp.dc, Level::High, Speed::Low);
        let lcd_reset = Output::new(dp.reset, Level::High, Speed::Low);
        let backlight = Output::new(dp.bl, Level::High, Speed::Low);
        let lcd_cs = Output::new(dp.cs, Level::High, Speed::Low);

        let lcd_spi_device = ExclusiveDevice::new(lcd_spi, lcd_cs, Delay).unwrap();

        let mut touch_config = spi::Config::default();
        touch_config.frequency = khz(200);
        touch_config.mode = spi::MODE_0;

        let touch_spi = Spi::new(
            tp.spi,
            tp.sck,
            tp.mosi,
            tp.miso,
            tp.dma_tx,
            tp.dma_rx,
            Irqs,
            touch_config,
        );

        let touch_cs = Output::new(tp.cs, Level::High, Speed::Low);
        let touch_irq = ExtiInput::new(tp.en, tp.irq, Pull::Up, Irqs);
        let touch_spi_device = ExclusiveDevice::new(touch_spi, touch_cs, Delay).unwrap();

        let touch = touch_controllers::new(touch_spi_device, touch_irq);
        let driver = drivers::new(lcd_spi_device, lcd_dc, lcd_reset);
        // let statem = Display::new_statem().expect("failed to create display state machine");

        Self {
            driver,
            touch,
            backlight_pin: backlight,
            cursor: 0,
            scroll_offset: 0,
            orientation,
            switch: false,
            // statem,
        }
    }

    pub async fn show_songs(&mut self) -> Result<(), DisplayError> {
        let text_style = MonoTextStyle::new(&FONT_7X14, Rgb565::WHITE);
        let path = ".";
        if self.cursor == 0 && self.scroll_offset == 0 {
            self.switch = false;
        };
        Rectangle::new(
            Point::new(0, 0),
            Size::new(self.width() as u32, self.height() as u32),
        )
        .draw_styled(
            &PrimitiveStyleBuilder::new()
                .fill_color(Rgb565::CSS_GREEN_YELLOW)
                .build(),
            &mut self.driver,
        )
        .unwrap();

        Rectangle::new(
            Point::new(0, (self.cursor * 14) as i32),
            Size::new(self.width() as u32, 16 as u32),
        )
        .draw_styled(
            &PrimitiveStyleBuilder::new().fill_color(Rgb565::RED).build(),
            &mut self.driver,
        )
        .unwrap();

        let list = FS::requests::list_dir(path, self.scroll_offset).await;
        match list {
            Ok(entries) => {
                if entries.len() < LS_ENTRIES && self.cursor == entries.len() - 1 {
                    self.switch = true;
                }
                let mut y = 14;
                for d in entries {
                    Text::new(d.name.as_str(), Point::new(10, y), text_style)
                        .draw(&mut self.driver)
                        .unwrap();
                    y += 14;
                }
                if self.switch {
                    self.scroll_up();
                } else {
                    self.scroll_down();
                };
                Ok(())
            }
            Err(e) => Err(DisplayError::from(e)),
        }
    }

    pub async fn init(&mut self) -> Result<(), LcdDriverError> {
        self.driver.init().await
    }

    pub async fn flush(&mut self) -> Result<(), LcdDriverError> {
        self.driver.flush().await
    }

    pub async fn clear_screen(&mut self, color: Rgb565) -> Result<(), LcdDriverError> {
        self.driver.clear_screen(color).await
    }

    pub async fn set_orientation(
        &mut self,
        orientation: Orientation,
    ) -> Result<(), LcdDriverError> {
        self.orientation = orientation;
        self.driver.set_orientation(orientation).await
    }

    pub fn scroll_down(&mut self) {
        self.cursor += 1;
        if self.cursor >= LS_ENTRIES {
            self.cursor = 0;
            self.scroll_offset += LS_ENTRIES; // page forward
        }
    }

    pub fn scroll_up(&mut self) {
        if self.cursor == 0 {
            if self.scroll_offset == 0 {
                return;
            }
            self.cursor = LS_ENTRIES;
            self.scroll_offset = self.scroll_offset.saturating_sub(LS_ENTRIES); // page forward
        }
        self.cursor = self.cursor.saturating_sub(1);
    }

    fn rotate_touch(&self, x: i32, y: i32) -> (i32, i32) {
        let w = WIDTH as i32;
        let h = HEIGHT as i32;

        match self.orientation {
            Orientation::Portrait => (x, y),
            Orientation::Landscape => (y, w - 1 - x),
            Orientation::PortraitFlipped => (w - 1 - x, h - 1 - y),
            Orientation::LandscapeFlipped => (h - 1 - y, x),
        }
    }

    pub async fn touch_read(&mut self) -> Option<(i32, i32)> {
        let (x, y) = self.touch.read().await?;
        Some(self.rotate_touch(x, y))
    }

    pub async fn touch_track(&mut self) -> Option<(i32, i32)> {
        let (x, y) = self.touch.track().await?;
        Some(self.rotate_touch(x, y))
    }

    pub fn backlight(&mut self, backlight: Backlight) {
        match backlight {
            Backlight::On => self.backlight_pin.set_high(),
            Backlight::Off => self.backlight_pin.set_low(),
            Backlight::Toggle => self.backlight_pin.toggle(),
        }
    }

    pub async fn power(&mut self, power: Power) -> Result<(), LcdDriverError> {
        match power {
            Power::On => {
                self.backlight(Backlight::On);
            }
            Power::Off => {
                self.backlight(Backlight::Off);
            }
        };
        self.driver.power(power).await
    }
}

#[embassy_executor::task]
pub async fn display_task(mut display: Display) {
    let _ = display.init().await;

    display.clear(Rgb565::BLACK).unwrap();
    display.flush().await.unwrap();
    let text_style = MonoTextStyle::new(&FONT_7X14, Rgb565::BLACK);

    loop {
        match DISP_REQ.receive().await {
            DisplayRequest::ShowSongs => {
                let reply = display
                    .show_songs()
                    .await
                    .map(|_| DisplayReply::ShowSongs)
                    .map_err(Into::into);
                DISP_REP.send(reply).await;
            }
            DisplayRequest::ShowPLayer => todo!(),
            DisplayRequest::GetOrientation => {
                let reply = Ok(DisplayReply::GetOrientation(display.orientation));
                DISP_REP.send(reply).await;
            }
            DisplayRequest::SetOrientation(orientation) => {
                let reply = display
                    .set_orientation(orientation)
                    .await
                    .map(|_| DisplayReply::SetOrientation)
                    .map_err(Into::into);

                let color = match orientation {
                    Orientation::Portrait => Rgb565::CSS_GREEN_YELLOW,
                    Orientation::Landscape => Rgb565::CSS_RED,
                    Orientation::PortraitFlipped => Rgb565::CSS_IVORY,
                    Orientation::LandscapeFlipped => Rgb565::CSS_THISTLE,
                };

                Rectangle::new(
                    Point::new(0, 0),
                    Size::new(display.width() as u32, display.height() as u32),
                )
                .draw_styled(
                    &PrimitiveStyleBuilder::new().fill_color(color).build(),
                    &mut display,
                )
                .unwrap();
                Text::new(orientation.as_str(), Point::new(50, 50), text_style)
                    .draw(&mut display)
                    .unwrap();
                display.flush().await.unwrap();
                DISP_REP.send(reply).await;
            }
        }
    }
}

const CAL_SAMPLES: i32 = 200;
const CAL_MARK: i32 = 6;

async fn sample_target(touch: &mut TouchController) -> (i32, i32, i32, i32) {
    touch.wait_touched().await;
    Timer::after_millis(50).await;

    let mut sum_x = 0i32;
    let mut sum_y = 0i32;
    let mut n = 0i32;
    let mut r_min = i32::MAX;
    let mut r_max = i32::MIN;

    while touch.is_touched() && n < CAL_SAMPLES {
        if let Ok(sample) = touch.read_raw().await {
            sum_x += sample.x;
            sum_y += sample.y;
            n += 1;
            r_min = r_min.min(sample.resistance);
            r_max = r_max.max(sample.resistance);
        }
        Timer::after_millis(5).await;
    }

    if n == 0 {
        return (0, 0, 0, 0);
    }
    (sum_x / n, sum_y / n, r_min, r_max)
}

fn draw_mark(display: &mut Display, x: i32, y: i32) {
    let style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb565::RED)
        .stroke_width(1)
        .build();

    Line::new(
        Point::new(x - CAL_MARK, y - CAL_MARK),
        Point::new(x + CAL_MARK, y + CAL_MARK),
    )
    .draw_styled(&style, display)
    .unwrap();

    Line::new(
        Point::new(x + CAL_MARK, y - CAL_MARK),
        Point::new(x - CAL_MARK, y + CAL_MARK),
    )
    .draw_styled(&style, display)
    .unwrap();
}

const PEN_RADIUS: i32 = 1;
const PEN_COLOR: Rgb565 = Rgb565::WHITE;

async fn dab(display: &mut Display, x: i32, y: i32) {
    let w = display.width() as i32;
    let h = display.height() as i32;

    let sx = (x - PEN_RADIUS).clamp(0, w - 1) as u16;
    let sy = (y - PEN_RADIUS).clamp(0, h - 1) as u16;
    let ex = (x + PEN_RADIUS).clamp(0, w - 1) as u16;
    let ey = (y + PEN_RADIUS).clamp(0, h - 1) as u16;

    if display
        .driver
        .fill_rect(sx, sy, ex, ey, PEN_COLOR)
        .await
        .is_err()
    {
        error!("failed to draw at {}, {}", x, y);
    }
}

async fn stroke(display: &mut Display, from: (i32, i32), to: (i32, i32)) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let steps = dx.abs().max(dy.abs());

    if steps == 0 {
        dab(display, to.0, to.1).await;
        return;
    }

    for i in 1..=steps {
        dab(display, from.0 + dx * i / steps, from.1 + dy * i / steps).await;
    }
}

#[embassy_executor::task]
pub async fn display_draw_task(mut display: Display) {
    let _ = display.init().await;
    display
        .set_orientation(Orientation::Landscape)
        .await
        .unwrap();
    display.clear_screen(Rgb565::BLACK).await.unwrap();

    loop {
        display.touch.wait_touched().await;

        let mut last = display.touch_read().await;
        if let Some((x, y)) = last {
            dab(&mut display, x, y).await;
        }

        while display.touch.is_touched() {
            let Some(point) = display.touch_track().await else {
                continue;
            };

            match last {
                Some(prev) => stroke(&mut display, prev, point).await,
                None => dab(&mut display, point.0, point.1).await,
            }

            last = Some(point);
        }
    }
}

#[embassy_executor::task]
pub async fn touch_calibration_task(mut display: Display) {
    let _ = display.init().await;
    display
        .set_orientation(Orientation::Portrait)
        .await
        .unwrap();

    let w = display.width() as i32;
    let h = display.height() as i32;

    let targets = [
        (w / 8, h / 8),
        (w / 8 * 7, h / 8),
        (w / 8, h / 8 * 7),
        (w / 8 * 7, h / 8 * 7),
    ];

    let mut touched = [(0i32, 0i32); 4];
    let mut r_lo = i32::MAX;
    let mut r_hi = i32::MIN;

    for i in 0..4 {
        let (tx, ty) = targets[i];

        display.clear(Rgb565::BLACK).unwrap();
        draw_mark(&mut display, tx, ty);
        display.flush().await.unwrap();

        let (rx, ry, rmin, rmax) = sample_target(&mut display.touch).await;
        touched[i] = (rx, ry);
        r_lo = r_lo.min(rmin);
        r_hi = r_hi.max(rmax);

        info!(
            "target {}: screen ({}, {}) -> raw ({}, {}), r {}..{}",
            i, tx, ty, rx, ry, rmin, rmax
        );

        while display.touch.is_touched() {
            Timer::after_millis(20).await;
        }
        Timer::after_millis(400).await;
    }

    let spread_x = (touched[0].0 - touched[1].0).abs();
    let spread_y = (touched[0].1 - touched[1].1).abs();
    let swap_xy = spread_y > spread_x;

    let (mut x1, mut x2, mut y1, mut y2) = if swap_xy {
        (
            (touched[0].1 + touched[2].1) / 2,
            (touched[1].1 + touched[3].1) / 2,
            (touched[0].0 + touched[1].0) / 2,
            (touched[2].0 + touched[3].0) / 2,
        )
    } else {
        (
            (touched[0].0 + touched[2].0) / 2,
            (touched[1].0 + touched[3].0) / 2,
            (touched[0].1 + touched[1].1) / 2,
            (touched[2].1 + touched[3].1) / 2,
        )
    };

    let edge_x = (x2 - x1) / 6;
    x1 -= edge_x;
    x2 += edge_x;

    let edge_y = (y2 - y1) / 6;
    y1 -= edge_y;
    y2 += edge_y;

    info!("---- paste into touch_controllers/mod.rs ----");
    info!("x1: {},", x1);
    info!("x2: {},", x2);
    info!("y1: {},", y1);
    info!("y2: {},", y2);
    info!("sx: {},", w);
    info!("sy: {},", h);
    info!("swap_xy: {},", swap_xy);
    info!("---- observed resistance range {}..{} ----", r_lo, r_hi);

    display.clear(Rgb565::BLACK).unwrap();
    display.flush().await.unwrap();

    loop {
        display.touch.wait_touched().await;
        if let Some((x, y)) = display.touch.read().await {
            info!("mapped x:{}, y:{}", x, y);
            Rectangle::new(Point::new(x - 1, y - 1), Size::new(3, 3))
                .draw_styled(
                    &PrimitiveStyleBuilder::new()
                        .fill_color(Rgb565::BLUE)
                        .build(),
                    &mut display,
                )
                .unwrap();
            display.flush().await.unwrap();
        }
    }
}

#[embassy_executor::task]
pub async fn display_taskt(mut display: Display) {
    let _ = display.init().await;

    display.clear(Rgb565::BLACK).unwrap();
    display.flush().await.unwrap();

    loop {
        display.touch.wait_touched().await;
        info!("after wait");
        if let Some((x, y)) = display.touch.read().await {
            info!("some points x:{}, y:{}", x, y);
            let style = PrimitiveStyleBuilder::new()
                .fill_color(Rgb565::BLUE)
                .build();

            Rectangle::new(Point::new(x - 1, y - 1), Size::new(3, 3))
                .into_styled(style)
                .draw(&mut display)
                .unwrap();

            display.flush().await.unwrap();
        } else {
            info!("no points");
        }
    }
}

/* #[embassy_executor::task]
pub async fn display_task(mut display: Display) {
    if !display.init().await.is_ok() {
        error!("failed to init display");
        return;
    }
    info!("display ready");

    let text_style = MonoTextStyle::new(&FONT_7X14, Rgb565::BLACK);

    let mut ori = Orientation::Portrait;
    loop {
        display.clear(Rgb565::BLACK).unwrap();
        display.set_orientation(ori).await.unwrap();

        let color = match ori {
            Orientation::Portrait => Rgb565::CSS_GREEN_YELLOW,
            Orientation::Landscape => Rgb565::CSS_RED,
            Orientation::PortraitFlipped => Rgb565::CSS_IVORY,
            Orientation::LandscapeFlipped => Rgb565::CSS_THISTLE,
        };

        Rectangle::new(
            Point::new(0, 0),
            Size::new(display.width() as u32, display.height() as u32),
        )
        .draw_styled(
            &PrimitiveStyleBuilder::new().fill_color(color).build(),
            &mut display,
        )
        .unwrap();
        Text::new(ori.as_str(), Point::new(50, 50), text_style)
            .draw(&mut display)
            .unwrap();

        ori = ori.next();

        display.flush().await.unwrap();
        Timer::after_millis(2000).await;
    }
} */

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        self.driver.size()
    }
}

impl DrawTarget for Display {
    type Color = Rgb565;
    type Error = <LcdDriver as DrawTarget>::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        self.driver.draw_iter(pixels)
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        self.driver.fill_solid(area, color)
    }
}

pub trait Dimensions: OriginDimensions {
    fn width(&self) -> u32 {
        self.size().width
    }
    fn height(&self) -> u32 {
        self.size().height
    }
}
impl<T: OriginDimensions> Dimensions for T {}

#[embassy_executor::task]
pub async fn display_taskx(mut display: Display) {
    if !display.init().await.is_ok() {
        error!("failed to init display");
        return;
    }
    info!("display ready");

    let text_style = MonoTextStyle::new(&FONT_7X14, Rgb565::WHITE);

    let mut switch = false;
    let mut keep_scrolling = true;
    let path = ".";
    loop {
        display.driver.clear(Rgb565::BLACK).unwrap();
        if display.cursor == 0 && display.scroll_offset == 0 {
            switch = false;
        };
        Rectangle::new(
            Point::new(0, 0),
            Size::new(display.width() as u32, display.height() as u32),
        )
        .draw_styled(
            &PrimitiveStyleBuilder::new()
                .fill_color(Rgb565::CSS_GREEN_YELLOW)
                .build(),
            &mut display.driver,
        )
        .unwrap();

        Rectangle::new(
            Point::new(0, (display.cursor * 14) as i32),
            Size::new(display.width() as u32, 16 as u32),
        )
        .draw_styled(
            &PrimitiveStyleBuilder::new().fill_color(Rgb565::RED).build(),
            &mut display.driver,
        )
        .unwrap();

        let list = FS::requests::list_dir(path, display.scroll_offset).await;
        match list {
            Ok(entries) => {
                if entries.len() < LS_ENTRIES && display.cursor == entries.len() - 1 {
                    switch = true;
                }
                let mut y = 14;
                for d in entries {
                    Text::new(d.name.as_str(), Point::new(10, y), text_style)
                        .draw(&mut display.driver)
                        .unwrap();
                    y += 14;
                }
            }
            Err(_) => {}
        }

        if switch {
            display.scroll_up();
        } else {
            display.scroll_down();
        }

        display.driver.flush().await.unwrap();
        Timer::after_millis(100).await;
    }
}

pub mod requests {
    pub enum DisplayRequest {
        GetOrientation,
        SetOrientation(Orientation),
        ShowSongs,
        ShowPLayer,
    }

    pub enum DisplayReply {
        GetOrientation(Orientation),
        SetOrientation,
        ShowSongs,
        ShowPlayer,
    }

    use super::*;

    pub async fn show_songs() -> Result<(), DisplayError> {
        DISP_REQ.send(DisplayRequest::ShowSongs).await;
        match DISP_REP.receive().await {
            Ok(DisplayReply::ShowSongs) => Ok(()),
            Ok(_) => unreachable!("display_task only replies ShowSongs to a ShowSongs request"),
            Err(e) => Err(e),
        }
    }

    pub async fn get_orientation() -> Result<Orientation, DisplayError> {
        DISP_REQ.send(DisplayRequest::GetOrientation).await;
        match DISP_REP.receive().await {
            Ok(DisplayReply::GetOrientation(o)) => Ok(o),
            Ok(_) => {
                unreachable!("display_task only replies GetOrientation to a GetOrientation request")
            }
            Err(e) => Err(e),
        }
    }

    pub async fn set_orientation(ori: Orientation) -> Result<(), DisplayError> {
        DISP_REQ.send(DisplayRequest::SetOrientation(ori)).await;
        match DISP_REP.receive().await {
            Ok(DisplayReply::SetOrientation) => Ok(()),
            Ok(_) => {
                unreachable!("display_task only replies SetOrientation to a SetOrientation request")
            }
            Err(e) => Err(e),
        }
    }
}
