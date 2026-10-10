#![no_std]

#[macro_use]
mod fmt;

pub mod day;
pub mod details;
pub mod forecast;
pub mod weather_icon;

use embedded_graphics::{
    image::{Image, ImageDrawable},
    mono_font::{
        MonoTextStyle,
        iso_8859_1::{FONT_7X14_BOLD, FONT_9X18_BOLD, FONT_10X20},
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
    details::{Conditions, Details},
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

    title_row("Lisbon", "Thursday", "Oct 10")
        .align_to(&display.bounding_box(), horizontal::Center, vertical::Top)
        .translate(Point::new(0, ROW_MARGIN))
        .draw(display)?;

    let details = Details::new(&Conditions {
        sunrise: 1_791_887_344,
        sunset: 1_791_828_203,
        timezone_offset: 3_600,
        wind_kmh: 23.8,
        humidity: 59,
        pressure_hpa: 1019.3,
        uvi: 4.9,
        dew_point_c: 12.6,
        rain_chance: 0.33,
    });

    details_grid(&details)
        .align_to(&display.bounding_box(), horizontal::Right, vertical::Center)
        .translate(Point::new(-ROW_MARGIN, -40))
        .draw(display)?;

    Ok(())
}

fn details_grid(details: &Details) -> impl Widget {
    let left = LinearLayout::vertical(
        Chain::new(cell(&details.sunrise.icon, &details.sunrise.text))
            .append(cell(&details.wind.icon, &details.wind.text))
            .append(cell(&details.pressure.icon, &details.pressure.text))
            .append(cell(&details.dew_point.icon, &details.dew_point.text)),
    )
    .with_alignment(horizontal::Left)
    .with_spacing(FixedMargin(ROW_MARGIN / 2))
    .arrange();

    let right = LinearLayout::vertical(
        Chain::new(cell(&details.sunset.icon, &details.sunset.text))
            .append(cell(&details.humidity.icon, &details.humidity.text))
            .append(cell(&details.uvi.icon, &details.uvi.text))
            .append(cell(&details.rain_chance.icon, &details.rain_chance.text)),
    )
    .with_alignment(horizontal::Left)
    .with_spacing(FixedMargin(ROW_MARGIN / 2))
    .arrange();

    LinearLayout::horizontal(Chain::new(left).append(right))
        .with_alignment(vertical::Top)
        .with_spacing(FixedMargin(ROW_MARGIN * 2))
        .arrange()
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

fn title_row(location: &str, week_day: &str, date: &str) -> impl Widget {
    let style = MonoTextStyle::new(&FONT_9X18_BOLD, TriColor::Red);
    let separator = || Text::new("|", Point::zero(), style);

    LinearLayout::horizontal(
        Chain::new(Text::new(location, Point::zero(), style))
            .append(separator())
            .append(Text::new(week_day, Point::zero(), style))
            .append(separator())
            .append(Text::new(date, Point::zero(), style)),
    )
    .with_alignment(vertical::Center)
    .with_spacing(FixedMargin(ROW_MARGIN))
    .arrange()
}

fn cell<I: ImageDrawable<Color = TriColor>>(icon: &I, text: &str) -> impl Widget {
    let style = MonoTextStyle::new(&FONT_7X14_BOLD, TriColor::Black);

    LinearLayout::horizontal(
        Chain::new(Image::new(icon, Point::zero())).append(Text::new(text, Point::zero(), style)),
    )
    .with_alignment(vertical::Center)
    .with_spacing(FixedMargin(ROW_MARGIN / 2))
    .arrange()
}

fn day_widget(forecast: &Forecast) -> impl Widget {
    let text_style = MonoTextStyle::new(&FONT_7X14_BOLD, TriColor::Red);
    let day = Text::new(forecast.week_day.as_short_str(), Point::zero(), text_style);
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
