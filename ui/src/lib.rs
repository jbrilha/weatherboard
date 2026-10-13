#![no_std]

#[macro_use]
mod fmt;

pub mod day;
pub mod details;
pub mod forecast;
pub mod today;
pub mod weather_icon;

use embedded_graphics::{
    image::{Image, ImageDrawable},
    mono_font::MonoTextStyle,
    prelude::*,
    primitives::{
        Circle, Line, Polyline, Primitive, PrimitiveStyle, Rectangle, RoundedRectangle,
        StyledDrawable,
    },
    text::Text,
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
    day::Day::{self},
    details::{Conditions, Details},
    forecast::Forecast,
    today::Today,
    weather_icon::{LargeWeatherIcon, WeatherIcon},
};

const ROW_MARGIN: i32 = 10;
const TODAY_SPACING: i32 = 15;
const DETAIL_SPACING: i32 = 10;
const HIGH_LOW_SPACING: i32 = 20;

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

const X_LINE_MIN: i32 = 30;
const X_LINE_MAX: i32 = 370;
const Y_LINE_TOP: i32 = 160;
const Y_LINE_BOT: i32 = 190;

const TEMP_POINTS: [Point; 12] = [
    Point::new(X_LINE_MIN, Y_LINE_BOT),
    Point::new(60, Y_LINE_BOT),
    Point::new(90, Y_LINE_BOT),
    Point::new(120, Y_LINE_TOP),
    Point::new(150, Y_LINE_TOP),
    Point::new(180, Y_LINE_TOP),
    Point::new(210, Y_LINE_TOP),
    Point::new(240, Y_LINE_TOP),
    Point::new(270, Y_LINE_BOT),
    Point::new(300, Y_LINE_BOT),
    Point::new(330, Y_LINE_BOT),
    Point::new(X_LINE_MAX, Y_LINE_BOT),
];

const POP_POINTS: [Point; 12] = [
    Point::new(X_LINE_MIN, Y_LINE_TOP),
    Point::new(60, Y_LINE_TOP),
    Point::new(90, Y_LINE_TOP),
    Point::new(120, Y_LINE_BOT),
    Point::new(150, Y_LINE_BOT),
    Point::new(180, Y_LINE_BOT),
    Point::new(210, Y_LINE_BOT),
    Point::new(240, Y_LINE_BOT),
    Point::new(270, Y_LINE_TOP),
    Point::new(300, Y_LINE_TOP),
    Point::new(330, Y_LINE_TOP),
    Point::new(X_LINE_MAX, Y_LINE_TOP),
];

pub fn iconoir_test<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    display.clear(TriColor::White)?;

    // temp_line().draw(display)?;
    // pop_line().draw(display)?;
    line_widget()
        .align_to(
            &display.bounding_box(),
            horizontal::Center,
            vertical::Center,
        )
        .translate(Point::new(0, 20))
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

fn today_widget(today: &Today) -> impl Widget {
    let temp_style = MonoTextStyle::new(&PROFONT_24_POINT, TriColor::Red);
    let feels_like_style = MonoTextStyle::new(&PROFONT_14_POINT, TriColor::Black);

    let temps = LinearLayout::vertical(
        Chain::new(Text::new(&today.temp_txt, Point::zero(), temp_style)).append(Text::new(
            &today.feels_like_txt,
            Point::zero(),
            feels_like_style,
        )),
    )
    .with_alignment(horizontal::Left)
    .with_spacing(FixedMargin(TODAY_SPACING / 2))
    .arrange();

    let divider = Line::new(
        Point::zero(),
        Point::new(0, today.icon.size().height as i32),
    )
    .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1));

    LinearLayout::horizontal(
        Chain::new(Image::new(&today.icon, Point::zero()))
            .append(temps)
            .append(divider),
    )
    .with_alignment(vertical::Center)
    .with_spacing(FixedMargin(TODAY_SPACING))
    .arrange()
}

fn temp_line() -> impl Widget {
    let line_style = PrimitiveStyle::with_stroke(TriColor::Red, 1);

    Polyline::new(&TEMP_POINTS).into_styled(line_style)
}

fn pop_line() -> impl Widget {
    let line_style = PrimitiveStyle::with_stroke(TriColor::Black, 1);

    Polyline::new(&POP_POINTS).into_styled(line_style)
}

fn line_widget() -> impl Widget {
    let temp_style = MonoTextStyle::new(&PROFONT_9_POINT, TriColor::Red);
    let high_temp = Text::new("36", Point::zero(), temp_style);
    let low_temp = Text::new("9", Point::zero(), temp_style);

    let pop_style = MonoTextStyle::new(&PROFONT_9_POINT, TriColor::Black);
    let high_pop = Text::new("100%", Point::zero(), pop_style);
    let low_pop = Text::new("0&", Point::zero(), pop_style);

    let temp = LinearLayout::vertical(Chain::new(high_temp).append(low_temp))
        .with_alignment(horizontal::Center)
        .with_spacing(FixedMargin(HIGH_LOW_SPACING))
        .arrange();

    let pop = LinearLayout::vertical(Chain::new(high_pop).append(low_pop))
        .with_alignment(horizontal::Center)
        .with_spacing(FixedMargin(HIGH_LOW_SPACING))
        .arrange();

    let lines = Chain::new(temp_line()).append(pop_line());

    let full = Chain::new(temp).append(lines).append(pop);

    let lines2 = LinearLayout::horizontal(full)
        .with_alignment(vertical::Center)
        .with_spacing(FixedMargin(6))
        .arrange();

    let fuller = Chain::new(lines2).append(lines).append(pop);

    let HOURS: [Text<'_, MonoTextStyle<'_, TriColor>>; 12] = [
        Text::new("8 AM", Point::zero(), pop_style),
        Text::new("10 AM", Point::zero(), pop_style),
        Text::new("12 PM", Point::zero(), pop_style),
        Text::new("2 PM", Point::zero(), pop_style),
        Text::new("4 PM", Point::zero(), pop_style),
        Text::new("6 PM", Point::zero(), pop_style),
        Text::new("8 PM", Point::zero(), pop_style),
        Text::new("10 PM", Point::zero(), pop_style),
        Text::new("12 AM", Point::zero(), pop_style),
        Text::new("2 AM", Point::zero(), pop_style),
        Text::new("4 AM", Point::zero(), pop_style),
        Text::new("6 AM", Point::zero(), pop_style),
    ];

    let hours = Chain::new(fuller).append(HOURS);

    LinearLayout::vertical(fuller)
        .with_alignment(vertical::Center)
        .with_spacing(FixedMargin(6))
        .arrange()
    // .translate(Point::new(0, 150))
}

fn details_grid(details: &Details) -> impl Widget {
    let left = LinearLayout::vertical(
        Chain::new(cell(&details.sunrise.icon, &details.sunrise.text))
            .append(cell(&details.wind.icon, &details.wind.text))
            .append(cell(&details.pressure.icon, &details.pressure.text))
            .append(cell(&details.dew_point.icon, &details.dew_point.text)),
    )
    .with_alignment(horizontal::Left)
    .with_spacing(FixedMargin(DETAIL_SPACING / 2))
    .arrange();

    let right = LinearLayout::vertical(
        Chain::new(cell(&details.sunset.icon, &details.sunset.text))
            .append(cell(&details.humidity.icon, &details.humidity.text))
            .append(cell(&details.uvi.icon, &details.uvi.text))
            .append(cell(&details.rain_chance.icon, &details.rain_chance.text)),
    )
    .with_alignment(horizontal::Left)
    .with_spacing(FixedMargin(DETAIL_SPACING / 2))
    .arrange();

    LinearLayout::horizontal(Chain::new(left).append(right))
        .with_alignment(vertical::Top)
        .with_spacing(FixedMargin(DETAIL_SPACING))
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
    let style = MonoTextStyle::new(&PROFONT_14_POINT, TriColor::Red);

    let divider = Line::new(Point::zero(), Point::new(0, 14))
        .into_styled(PrimitiveStyle::with_stroke(TriColor::Red, 1));

    LinearLayout::horizontal(
        Chain::new(Text::new(location, Point::zero(), style))
            // .append(separator())
            .append(divider)
            .append(Text::new(week_day, Point::zero(), style))
            // .append(separator())
            .append(divider)
            .append(Text::new(date, Point::zero(), style)),
    )
    .with_alignment(vertical::Center)
    .with_spacing(FixedMargin(ROW_MARGIN))
    .arrange()
}

fn cell<I: ImageDrawable<Color = TriColor>>(icon: &I, text: &str) -> impl Widget {
    let style = MonoTextStyle::new(&PROFONT_12_POINT, TriColor::Black);

    LinearLayout::horizontal(
        Chain::new(Image::new(icon, Point::zero())).append(Text::new(text, Point::zero(), style)),
    )
    .with_alignment(vertical::Center)
    .with_spacing(FixedMargin(ROW_MARGIN / 2))
    .arrange()
}

fn day_widget(forecast: &Forecast) -> impl Widget {
    let text_style = MonoTextStyle::new(&PROFONT_10_POINT, TriColor::Red);
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
