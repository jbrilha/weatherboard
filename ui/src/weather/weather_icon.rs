use common::weather::Condition;
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    image::ImageDrawable,
    primitives::Rectangle,
};
use embedded_iconoir::{Icon, size48px, size96px};
use weact_studio_epd::TriColor;

macro_rules! with_icon {
    ($enum:ident, $self:expr, $icon:ident => $body:expr) => {
        match $self {
            $enum::Sunny($icon) => $body,
            $enum::PartlyCloudy($icon) => $body,
            $enum::Cloudy($icon) => $body,
            $enum::Fog($icon) => $body,
            $enum::Rain($icon) => $body,
            $enum::HeavyRain($icon) => $body,
            $enum::Snow($icon) => $body,
            $enum::Thunderstorm($icon) => $body,
            $enum::Wind($icon) => $body,
        }
    };
}

macro_rules! weather_icon {
    ($name:ident, $size:ident) => {
        pub enum $name {
            Sunny(Icon<TriColor, $size::weather::SunLight>),
            PartlyCloudy(Icon<TriColor, $size::weather::CloudSunny>),
            Cloudy(Icon<TriColor, $size::weather::Cloud>),
            Fog(Icon<TriColor, $size::weather::Fog>),
            Rain(Icon<TriColor, $size::weather::Rain>),
            HeavyRain(Icon<TriColor, $size::weather::HeavyRain>),
            Snow(Icon<TriColor, $size::weather::Snow>),
            Thunderstorm(Icon<TriColor, $size::weather::Thunderstorm>),
            Wind(Icon<TriColor, $size::weather::Wind>),
        }

        impl $name {
            pub fn new(condition: Condition, color: TriColor) -> Self {
                match condition {
                    Condition::Sunny => Self::Sunny(Icon::new(color)),
                    Condition::PartlyCloudy => Self::PartlyCloudy(Icon::new(color)),
                    Condition::Cloudy => Self::Cloudy(Icon::new(color)),
                    Condition::Fog => Self::Fog(Icon::new(color)),
                    Condition::Rain => Self::Rain(Icon::new(color)),
                    Condition::HeavyRain => Self::HeavyRain(Icon::new(color)),
                    Condition::Snow => Self::Snow(Icon::new(color)),
                    Condition::Thunderstorm => Self::Thunderstorm(Icon::new(color)),
                    Condition::Wind => Self::Wind(Icon::new(color)),
                }
            }
        }

        impl OriginDimensions for $name {
            fn size(&self) -> Size {
                with_icon!($name, self, icon => icon.size())
            }
        }

        impl ImageDrawable for $name {
            type Color = TriColor;

            fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
            where
                D: DrawTarget<Color = TriColor>,
            {
                with_icon!($name, self, icon => icon.draw(target))
            }

            fn draw_sub_image<D>(&self, target: &mut D, area: &Rectangle) -> Result<(), D::Error>
            where
                D: DrawTarget<Color = TriColor>,
            {
                with_icon!($name, self, icon => icon.draw_sub_image(target, area))
            }
        }
    };
}

weather_icon!(WeatherIcon, size48px);
weather_icon!(LargeWeatherIcon, size96px);
