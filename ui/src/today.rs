use core::fmt::Write;

use heapless::String;

use crate::weather_icon::LargeWeatherIcon;

pub struct Today {
    pub icon: LargeWeatherIcon,
    pub temp_txt: String<5>,
    pub feels_like_txt: String<5>,
}

impl Today {
    pub fn new(icon: LargeWeatherIcon, temp_c: f32, feels_like_c: f32) -> Self {
        let mut temp_txt = String::new();
        write!(temp_txt, "{temp_c:.0}°C").unwrap();

        let mut feels_like_txt = String::new();
        write!(feels_like_txt, "{feels_like_c:.0}°C").unwrap();

        Self {
            icon,
            temp_txt,
            feels_like_txt,
        }
    }
}
