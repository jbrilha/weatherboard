use core::fmt::Write;

use embedded_graphics::{
    geometry::{OriginDimensions, Point},
    image::Image,
    mono_font::MonoTextStyle,
    primitives::{Line, Primitive, PrimitiveStyle},
    text::Text,
};
use embedded_layout::{
    align::{horizontal, vertical},
    layout::linear::{FixedMargin, LinearLayout},
    object_chain::Chain,
};
use heapless::String;
use profont::{PROFONT_14_POINT, PROFONT_24_POINT};
use weact_studio_epd::TriColor;

use crate::Widget;
use crate::weather::weather_icon::LargeWeatherIcon;

const TODAY_SPACING: i32 = 15;

pub struct Today {
    pub icon: LargeWeatherIcon,
    pub temp_txt: String<5>,
    pub feels_like_txt: String<5>,
}

impl Today {
    pub fn new(icon: LargeWeatherIcon, temp_c: i8, feels_like_c: i8) -> Self {
        let mut temp_txt = String::new();
        write!(temp_txt, "{temp_c}°C").unwrap();

        let mut feels_like_txt = String::new();
        write!(feels_like_txt, "{feels_like_c}°C").unwrap();

        Self {
            icon,
            temp_txt,
            feels_like_txt,
        }
    }
}

pub fn today_widget(today: &Today) -> impl Widget {
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
