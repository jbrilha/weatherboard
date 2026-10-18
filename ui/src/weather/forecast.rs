use core::fmt::Write;

use heapless::String;

use crate::weather::{day::Day, weather_icon::WeatherIcon};

pub struct Forecast {
    pub week_day: Day,
    pub icon: WeatherIcon,
    pub temperature_str: String<8>,
}

impl Forecast {
    pub fn new(week_day: Day, icon: WeatherIcon, low_temp: i8, high_temp: i8) -> Self {
        let mut temperature_str = String::new();
        write!(temperature_str, " {low_temp}/{high_temp}°").unwrap();

        Self {
            week_day,
            icon,
            temperature_str,
        }
    }
}
