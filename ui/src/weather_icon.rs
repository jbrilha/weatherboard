use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    image::ImageDrawable,
    primitives::Rectangle,
};
use embedded_iconoir::{Icon, size48px::weather};
use weact_studio_epd::TriColor;

use crate::day::Day;

pub enum WeatherIcon {
    Sunny(Icon<TriColor, weather::SunLight>),
    PartlyCloudy(Icon<TriColor, weather::CloudSunny>),
    Cloudy(Icon<TriColor, weather::Cloud>),
    Fog(Icon<TriColor, weather::Fog>),
    Rain(Icon<TriColor, weather::Rain>),
    HeavyRain(Icon<TriColor, weather::HeavyRain>),
    Snow(Icon<TriColor, weather::Snow>),
    Thunderstorm(Icon<TriColor, weather::Thunderstorm>),
    Wind(Icon<TriColor, weather::Wind>),
}

macro_rules! with_icon {
    ($self:expr, $icon:ident => $body:expr) => {
        match $self {
            WeatherIcon::Sunny($icon) => $body,
            WeatherIcon::PartlyCloudy($icon) => $body,
            WeatherIcon::Cloudy($icon) => $body,
            WeatherIcon::Fog($icon) => $body,
            WeatherIcon::Rain($icon) => $body,
            WeatherIcon::HeavyRain($icon) => $body,
            WeatherIcon::Snow($icon) => $body,
            WeatherIcon::Thunderstorm($icon) => $body,
            WeatherIcon::Wind($icon) => $body,
        }
    };
}

impl OriginDimensions for WeatherIcon {
    fn size(&self) -> Size {
        with_icon!(self, icon => icon.size())
    }
}

impl ImageDrawable for WeatherIcon {
    type Color = TriColor;

    fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = TriColor>,
    {
        with_icon!(self, icon => icon.draw(target))
    }

    fn draw_sub_image<D>(&self, target: &mut D, area: &Rectangle) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = TriColor>,
    {
        with_icon!(self, icon => icon.draw_sub_image(target, area))
    }
}
