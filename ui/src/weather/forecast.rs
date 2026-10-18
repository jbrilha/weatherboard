use core::fmt::Write;

use heapless::String;

use crate::weather::{day::Day, weather_icon::WeatherIcon};

pub struct Forecast {
    pub week_day: Day,
    pub icon: WeatherIcon,
    pub low_temp: i8,
    pub high_temp: i8,
    pub temp_txt: String<16>,
}

impl Forecast {
    pub fn new(week_day: Day, icon: WeatherIcon, low_temp: i8, high_temp: i8) -> Self {
        let mut temp_txt = String::new();
        write!(temp_txt, " {low_temp}/{high_temp}°").unwrap();

        Self {
            week_day,
            icon,
            low_temp,
            high_temp,
            temp_txt,
        }
    }
}
