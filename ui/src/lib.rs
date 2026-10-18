#![no_std]

#[macro_use]
mod fmt;

mod chart;
mod day;
mod details;
mod forecast;
mod title;
mod today;
mod weather_icon;

use embedded_graphics::{
    image::{Image, ImageDrawable},
    mono_font::MonoTextStyle,
    prelude::*,
    primitives::{
        Circle, Line, Polyline, Primitive, PrimitiveStyle, Rectangle, RoundedRectangle,
        StyledDrawable,
    },
    text::{Alignment, Text},
};
use embedded_iconoir::{
    icons::{size48px::weather, size96px},
    prelude::IconoirNewIcon,
};
use embedded_layout::{
    View,
    align::{Align, horizontal, vertical},
    layout::linear::{FixedMargin, LinearLayout},
    object_chain::Chain,
};
use enum_iterator::next_cycle;
use profont::{
    PROFONT_7_POINT, PROFONT_9_POINT, PROFONT_10_POINT, PROFONT_12_POINT, PROFONT_14_POINT,
    PROFONT_24_POINT,
};
use rand::{RngExt, SeedableRng, rngs::SmallRng};
pub use weact_studio_epd::TriColor;

use crate::{
    chart::{Chart, chart_widget},
    day::{
        Day::{self},
        week_widget,
    },
    details::{Conditions, Details, details_grid},
    forecast::Forecast,
    title::{Title, title_row},
    today::{Today, today_widget},
    weather_icon::{LargeWeatherIcon, WeatherIcon},
};

const ROW_MARGIN: i32 = 10;

pub trait Widget: View + Drawable<Color = TriColor> {}
impl<T: View + Drawable<Color = TriColor>> Widget for T {}
const HOUR0: usize = 8;

pub fn iconoir_test<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    display.clear(TriColor::White)?;

    // temp_line().draw(display)?;
    // pop_line().draw(display)?;
    let chart = Chart::new(core::array::from_fn(|i| ((HOUR0 + 2 * i) % 24) as u8));

    chart_widget(&chart)
        .align_to(
            &display.bounding_box(),
            horizontal::Center,
            vertical::Center,
        )
        .translate(Point::new(0, 25))
        .draw(display)?;

    /* let (mid_h, mid_v) = (
        display.bounding_box().size().height / 2,
        display.bounding_box().size().width / 2,
    );
    let divider_v = Line::new(
        Point::new(mid_v as i32, 0),
        Point::new(mid_v as i32, display.bounding_box().size().height as i32),
    )
    .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
    .draw(display)?;
    let divider_h = Line::new(
        Point::new(0, mid_h as i32),
        Point::new(display.bounding_box().size().width as i32, mid_h as i32),
    )
    .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
    .draw(display)?; */

    let (primary, secondary) = (TriColor::Black, TriColor::Red);

    let mut rng = SmallRng::seed_from_u64(12);
    let mut day = Day::Thursday;
    let forecasts = [
        WeatherIcon::Sunny(weather::SunLight::new(primary)),
        WeatherIcon::PartlyCloudy(weather::CloudSunny::new(primary)),
        WeatherIcon::Cloudy(weather::Cloud::new(primary)),
        WeatherIcon::Rain(weather::Rain::new(primary)),
        WeatherIcon::Thunderstorm(weather::Thunderstorm::new(primary)),
        WeatherIcon::Snow(weather::Snow::new(primary)),
    ]
    .map(|icon| {
        day = next_cycle(&day);
        let low_temp: i8 = rng.random_range(10..=99);
        let high_temp: i8 = rng.random_range(10..=99);
        forecast::Forecast::new(day, icon, low_temp, high_temp)
    });

    week_widget(&forecasts)
        .align_to(
            &display.bounding_box(),
            horizontal::Center,
            vertical::Bottom,
        )
        .translate(Point::new(0, -ROW_MARGIN / 2))
        .draw(display)?;

    let title = Title::new("Lisbon", Day::Thursday, "Oct 10");

    title_row(&title)
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
        .translate(Point::new(-ROW_MARGIN, -60))
        .draw(display)?;

    let today = Today::new(
        LargeWeatherIcon::PartlyCloudy(size96px::weather::CloudSunny::new(primary)),
        20.7,
        19.9,
    );

    today_widget(&today)
        .align_to(&display.bounding_box(), horizontal::Left, vertical::Center)
        .translate(Point::new(ROW_MARGIN, -60))
        .draw(display)?;

    Ok(())
}

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
        MonoTextStyle::new(&PROFONT_14_POINT, primary),
    )
    .draw(display)?;
    Text::new(
        "black red white",
        Point::new(10, 48),
        MonoTextStyle::new(&PROFONT_14_POINT, secondary),
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

    Rectangle::new(Point::new(320, 170), Size::new(65, 120))
        .into_styled(PrimitiveStyle::with_stroke(secondary, 3))
        .draw(display)?;

    Ok(())
}
