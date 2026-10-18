use core::fmt::Write;

use embedded_graphics::{
    geometry::Point,
    image::{Image, ImageDrawable},
    mono_font::MonoTextStyle,
    text::Text,
};
use embedded_iconoir::{
    Icon,
    prelude::{IconoirIcon, IconoirNewIcon},
    size24px::{
        clothing::UmbrellaFull,
        design_tools::Droplet,
        other::HalfMoon,
        system::DashboardSpeed,
        weather::{DewPoint, SunLight, Wind},
    },
};
use embedded_layout::{
    align::{horizontal, vertical},
    layout::linear::{FixedMargin, LinearLayout},
    object_chain::Chain,
};
use heapless::String;
use profont::PROFONT_12_POINT;
use weact_studio_epd::TriColor;

use crate::{ROW_MARGIN, Widget};

const DETAIL_SPACING: i32 = 10;

pub struct Detail<I: IconoirIcon> {
    pub icon: Icon<TriColor, I>,
    pub text: String<12>,
}

impl<I: IconoirIcon + IconoirNewIcon<TriColor>> Detail<I> {
    fn new(args: core::fmt::Arguments) -> Self {
        let mut text = String::new();
        text.write_fmt(args).unwrap();
        Self {
            icon: I::new(TriColor::Black),
            text,
        }
    }
}

pub struct Conditions {
    pub sunrise: i64,
    pub sunset: i64,
    pub timezone_offset: i32,
    pub wind_kmh: f32,
    pub humidity: u8,
    pub pressure_hpa: f32,
    pub uvi: f32,
    pub dew_point_c: f32,
    pub rain_chance: f32,
}

pub struct Details {
    pub sunrise: Detail<SunLight>,
    pub sunset: Detail<HalfMoon>,
    pub wind: Detail<Wind>,
    pub humidity: Detail<Droplet>,
    pub pressure: Detail<DashboardSpeed>,
    pub uvi: Detail<SunLight>,
    pub dew_point: Detail<DewPoint>,
    pub rain_chance: Detail<UmbrellaFull>,
}

fn hour_minute(unix: i64, timezone_offset: i32) -> (i64, i64) {
    let secs = (unix + i64::from(timezone_offset)).rem_euclid(86_400);
    (secs / 3_600, secs % 3_600 / 60)
}

impl Details {
    pub fn new(c: &Conditions) -> Self {
        let (rise_h, rise_m) = hour_minute(c.sunrise, c.timezone_offset);
        let (set_h, set_m) = hour_minute(c.sunset, c.timezone_offset);

        Self {
            sunrise: Detail::new(format_args!("{rise_h:02}:{rise_m:02}")),
            sunset: Detail::new(format_args!("{set_h:02}:{set_m:02}")),
            wind: Detail::new(format_args!("{:.0} km/h", c.wind_kmh)),
            humidity: Detail::new(format_args!("{}%", c.humidity)),
            pressure: Detail::new(format_args!("{:.0} hPa", c.pressure_hpa)),
            uvi: Detail::new(format_args!("UV {:.1}", c.uvi)),
            dew_point: Detail::new(format_args!("{:.0}°C", c.dew_point_c)),
            rain_chance: Detail::new(format_args!("{:.0}%", c.rain_chance * 100.0)),
        }
    }
}

pub fn details_grid(details: &Details) -> impl Widget {
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

fn cell<I: ImageDrawable<Color = TriColor>>(icon: &I, text: &str) -> impl Widget {
    let style = MonoTextStyle::new(&PROFONT_12_POINT, TriColor::Black);

    LinearLayout::horizontal(
        Chain::new(Image::new(icon, Point::zero())).append(Text::new(text, Point::zero(), style)),
    )
    .with_alignment(vertical::Center)
    .with_spacing(FixedMargin(ROW_MARGIN / 2))
    .arrange()
}
