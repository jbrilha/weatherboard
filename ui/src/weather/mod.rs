mod chart;
mod day;
mod details;
mod forecast;
mod title;
mod today;
mod weather_icon;

use embedded_graphics::{
    prelude::*,
    primitives::{Line, PrimitiveStyle},
};
use embedded_iconoir::{
    icons::{size48px::weather, size96px},
    prelude::IconoirNewIcon,
};
use embedded_layout::{
    View,
    align::{Align, horizontal, vertical},
};
use enum_iterator::next_cycle;
use rand::{RngExt, SeedableRng, rngs::SmallRng};
pub use weact_studio_epd::TriColor;

use crate::weather::{
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
const HOUR0: usize = 8;

pub struct WeatherInfo {
    pub title: Title,
    pub today: Today,
    pub details: Details,
    pub chart: Chart,
    pub weekly_forecast: [Forecast; 6],
}

impl WeatherInfo {
    pub fn new(
        title: Title,
        today: Today,
        details: Details,
        chart: Chart,
        weekly_forecast: [Forecast; 6],
    ) -> Self {
        Self {
            title,
            today,
            details,
            chart,
            weekly_forecast,
        }
    }

    pub fn default() -> Self {
        let mut day = Day::Thursday;

        let title = Title::new("Lisbon", day, "Oct 10");
        let today = Today::new(
            LargeWeatherIcon::PartlyCloudy(size96px::weather::CloudSunny::new(TriColor::Red)),
            20.7,
            19.9,
        );
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
        let chart = Chart::new(core::array::from_fn(|i| ((HOUR0 + 2 * i) % 24) as u8));

        let mut rng = SmallRng::seed_from_u64(12);
        let weekly_forecast = [
            WeatherIcon::Sunny(weather::SunLight::new(TriColor::Black)),
            WeatherIcon::PartlyCloudy(weather::CloudSunny::new(TriColor::Black)),
            WeatherIcon::Cloudy(weather::Cloud::new(TriColor::Black)),
            WeatherIcon::Rain(weather::Rain::new(TriColor::Black)),
            WeatherIcon::Thunderstorm(weather::Thunderstorm::new(TriColor::Black)),
            WeatherIcon::Snow(weather::Snow::new(TriColor::Black)),
        ]
        .map(|icon| {
            day = next_cycle(&day);
            let low_temp: i8 = rng.random_range(10..=99);
            let high_temp: i8 = rng.random_range(10..=99);
            forecast::Forecast::new(day, icon, low_temp, high_temp)
        });

        WeatherInfo::new(title, today, details, chart, weekly_forecast)
    }
}

pub fn dashboard<D>(display: &mut D, info: &WeatherInfo) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    display.clear(TriColor::White)?;

    // draw_crosshair(display)?;

    title_row(&info.title)
        .align_to(&display.bounding_box(), horizontal::Center, vertical::Top)
        .translate(Point::new(0, ROW_MARGIN))
        .draw(display)?;

    today_widget(&info.today)
        .align_to(&display.bounding_box(), horizontal::Left, vertical::Center)
        .translate(Point::new(ROW_MARGIN, -60))
        .draw(display)?;

    details_grid(&info.details)
        .align_to(&display.bounding_box(), horizontal::Right, vertical::Center)
        .translate(Point::new(-ROW_MARGIN, -60))
        .draw(display)?;

    chart_widget(&info.chart)
        .align_to(
            &display.bounding_box(),
            horizontal::Center,
            vertical::Center,
        )
        .translate(Point::new(0, 25))
        .draw(display)?;

    week_widget(&info.weekly_forecast)
        .align_to(
            &display.bounding_box(),
            horizontal::Center,
            vertical::Bottom,
        )
        .translate(Point::new(0, -ROW_MARGIN / 2))
        .draw(display)?;

    Ok(())
}

pub fn draw_crosshair<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    let (mid_h, mid_v) = (
        display.bounding_box().size().height / 2,
        display.bounding_box().size().width / 2,
    );

    Line::new(
        Point::new(mid_v as i32, 0),
        Point::new(mid_v as i32, display.bounding_box().size().height as i32),
    )
    .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
    .draw(display)?;

    Line::new(
        Point::new(0, mid_h as i32),
        Point::new(display.bounding_box().size().width as i32, mid_h as i32),
    )
    .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
    .draw(display)?;

    Ok(())
}
