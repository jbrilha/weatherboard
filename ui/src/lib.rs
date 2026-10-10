#![no_std]

#[macro_use]
mod fmt;

pub mod day;
pub mod forecast;
pub mod weather_icon;

use embedded_graphics::{
    image::Image,
    mono_font::{
        MonoTextStyle,
        iso_8859_1::{FONT_7X14_BOLD, FONT_10X20},
    },
    prelude::*,
    primitives::{
        Circle, Line, Primitive, PrimitiveStyle, Rectangle, RoundedRectangle, StyledDrawable,
    },
    text::Text,
};
use embedded_iconoir::{icons::size48px::weather, prelude::IconoirNewIcon};
use embedded_layout::{
    View,
    align::{Align, horizontal, vertical},
    layout::linear::{FixedMargin, LinearLayout},
    object_chain::Chain,
};
use enum_iterator::next_cycle;
use rand::{RngExt, SeedableRng, rngs::SmallRng};
pub use weact_studio_epd::TriColor;

use crate::{
    day::Day::{self},
    forecast::Forecast,
    weather_icon::WeatherIcon,
};

const ROW_MARGIN: i32 = 8;

pub trait Widget: View + Drawable<Color = TriColor> {}
impl<T: View + Drawable<Color = TriColor>> Widget for T {}

pub fn test_pattern<D>(display: &mut D, label: &str, round: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    display.clear(TriColor::White)?;

    let (primary, secondary) = if round % 2 == 0 {
        (TriColor::Black, TriColor::Red)
    } else {
        (TriColor::Red, TriColor::Black)
    };

    Text::new(
        label,
        Point::new(10, 24),
        MonoTextStyle::new(&FONT_10X20, primary),
    )
    .draw(display)?;
    Text::new(
        "black red white",
        Point::new(10, 48),
        MonoTextStyle::new(&FONT_10X20, secondary),
    )
    .draw(display)?;

    let swatches = [TriColor::Black, TriColor::Red, TriColor::White];
    for (i, color) in swatches.into_iter().enumerate() {
        let top_left = Point::new(10 + i as i32 * 130, 64);
        Rectangle::new(top_left, Size::new(120, 90))
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)?;
        Rectangle::new(top_left, Size::new(120, 90))
            .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
            .draw(display)?;
    }

    for i in 0..40 {
        let x = 10 + i * 4;
        let color = if i % 2 == 0 { primary } else { secondary };
        Line::new(Point::new(x, 170), Point::new(x, 290))
            .into_styled(PrimitiveStyle::with_stroke(color, 1))
            .draw(display)?;
    }

    Circle::new(Point::new(190, 170), 120)
        .into_styled(PrimitiveStyle::with_fill(secondary))
        .draw(display)?;
    Circle::new(Point::new(220, 200), 60)
        .into_styled(PrimitiveStyle::with_fill(primary))
        .draw(display)?;

    Rectangle::new(Point::new(320, 170), Size::new(70, 120))
        .into_styled(PrimitiveStyle::with_stroke(secondary, 3))
        .draw(display)?;

    Ok(())
}

pub fn iconoir_test<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    display.clear(TriColor::White)?;

    let (primary, secondary) = (TriColor::Black, TriColor::Red);

    // border(display, secondary)?;

    // let icon_tiny = icons::size12px::weather::Cloud::new(primary);
    // let icon_normal = icons::size24px::weather::CloudSunny::new(secondary);
    // let icon_large = icons::size48px::weather::Rain::new(primary);
    // let icon_huge = icons::size144px::weather::SnowFlake::new(secondary);
    //
    // let image_tiny = Image::new(&icon_tiny, Point::new(10, 10));
    // let image_normal = Image::new(&icon_normal, Point::new(30, 10));
    // let image_large = Image::new(&icon_large, Point::new(60, 10));
    // let image_huge = Image::new(&icon_huge, Point::new(120, 10));
    //
    // image_tiny.draw(display)?;
    // image_normal.draw(display)?;
    // image_large.draw(display)?;
    // image_huge.draw(display)?;

    /* day_widget(
        Day::Friday,
        Point::new(100, 100),
        &icons::size48px::weather::Cloud::new(primary),
    )
    .draw(display)?;
    day_widget(
        Day::Monday,
        Point::new(200, 200),
        &icons::size48px::weather::Rain::new(primary),
    )
    .draw(display)?; */
    let mut rng = SmallRng::seed_from_u64(123);
    let mut day = Day::Thursday;
    let forecasts = [
        WeatherIcon::Sunny(weather::SunLight::new(primary)),
        WeatherIcon::PartlyCloudy(weather::CloudSunny::new(primary)),
        WeatherIcon::Cloudy(weather::Cloud::new(primary)),
        WeatherIcon::Rain(weather::Rain::new(primary)),
        WeatherIcon::Thunderstorm(weather::Thunderstorm::new(secondary)),
        WeatherIcon::Snow(weather::Snow::new(primary)),
    ]
    .map(|icon| {
        day = next_cycle(&day);
        let low_temp: i8 = rng.random_range(-99..=99);
        let high_temp: i8 = rng.random_range(-99..=99);
        forecast::Forecast::new(day, icon, low_temp, high_temp)
    });

    week_widget(&forecasts)
        .align_to(
            &display.bounding_box(),
            horizontal::Center,
            vertical::Bottom,
        )
        .translate(Point::new(0, -ROW_MARGIN))
        .draw(display)?;
    Ok(())
}

fn border<D>(display: &mut D, color: TriColor) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    display
        .bounding_box()
        .draw_styled(&PrimitiveStyle::with_stroke(color, 3), display)?;

    Ok(())
}

fn day_widget(forecast: &Forecast) -> impl Widget {
    let text_style = MonoTextStyle::new(&FONT_7X14_BOLD, TriColor::Red);
    let day = Text::new(forecast.day.as_short_str(), Point::zero(), text_style);
    let temp = Text::new(&forecast.temp_txt, Point::zero(), text_style);

    let img = Image::new(&forecast.icon, Point::zero());

    let content = LinearLayout::vertical(Chain::new(day).append(img).append(temp))
        .with_alignment(horizontal::Center)
        .with_spacing(FixedMargin(ROW_MARGIN / 2))
        .arrange()
        .translate(Point::zero());

    let frame = RoundedRectangle::with_equal_corners(content.bounds().offset(4), Size::new(6, 6))
        .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1));

    Chain::new(frame).append(content)
}

// 6 because I want to show 'today' up top
fn week_widget(forecasts: &[Forecast; 6]) -> impl Widget {
    let widget = |i: usize| day_widget(&forecasts[i]);

    LinearLayout::horizontal(
        Chain::new(widget(0))
            .append(widget(1))
            .append(widget(2))
            .append(widget(3))
            .append(widget(4))
            .append(widget(5)),
    )
    .with_alignment(vertical::Center)
    .with_spacing(FixedMargin(ROW_MARGIN))
    .arrange()
}
