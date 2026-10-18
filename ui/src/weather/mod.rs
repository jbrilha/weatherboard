mod chart;
mod day;
mod details;
mod forecast;
mod title;
mod today;
mod weather_icon;

use common::weather::{Location, Month, WeatherPayload};
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
    details::{Conditions, Details, HoursMinutes, details_grid},
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

        let title = Title::new(Location::Lisbon, day, Month::October, 10);
        let today = Today::new(
            LargeWeatherIcon::PartlyCloudy(size96px::weather::CloudSunny::new(TriColor::Black)),
            21,
            20,
        );
        let details = Details::new(&Conditions {
            sunrise: HoursMinutes::new(7, 15),
            sunset: HoursMinutes::new(18, 55),
            // sunrise: 1_791_887_344,
            // sunset: 1_791_828_203,
            // timezone_offset: 3_600,
            wind_kmh: 24,
            humidity_pct: 59,
            pressure_hpa: 1019,
            uvi_tenths: 49,
            dew_point_c: 13,
            rain_pct: 33,
        });
        let chart = Chart::new(
            19,
            36,
            core::array::from_fn(|i| ((HOUR0 + 2 * i) % 24) as u8),
            [19, 20, 22, 25, 28, 31, 33, 36, 32, 28, 24, 21],
            [0, 0, 5, 10, 20, 40, 60, 40, 20, 10, 0, 0],
        );

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

impl From<&WeatherPayload> for WeatherInfo {
    fn from(p: &WeatherPayload) -> Self {
        let title = Title::new(p.location, p.week_day, p.month, p.day);

        let today = Today::new(
            LargeWeatherIcon::new(p.condition, TriColor::Black),
            p.temp_c,
            p.feels_like_c,
        );

        let details = Details::new(&Conditions {
            sunrise: HoursMinutes::new(p.sunrise.0, p.sunrise.1),
            sunset: HoursMinutes::new(p.sunset.0, p.sunset.1),
            wind_kmh: p.wind_kmh,
            humidity_pct: p.humidity_pct,
            pressure_hpa: p.pressure_hpa,
            uvi_tenths: p.uvi_tenths,
            dew_point_c: p.dew_point_c,
            rain_pct: p.rain_pct,
        });

        let chart = Chart::new(
            p.chart_low_c,
            p.chart_high_c,
            core::array::from_fn(|i| ((usize::from(p.chart_start_hour) + 2 * i) % 24) as u8),
            p.chart_temps_c,
            p.chart_pop_pct,
        );

        let mut day = p.week_day;
        let weekly_forecast = p.forecast.map(|f| {
            day = next_cycle(&day);
            Forecast::new(
                day,
                WeatherIcon::new(f.condition, TriColor::Black),
                f.low_c,
                f.high_c,
            )
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
