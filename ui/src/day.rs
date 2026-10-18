use enum_iterator::Sequence;

use embedded_graphics::{
    image::Image,
    mono_font::MonoTextStyle,
    prelude::*,
    primitives::{Primitive, PrimitiveStyle, RoundedRectangle},
    text::Text,
};
use embedded_layout::{
    View,
    align::{horizontal, vertical},
    layout::linear::{FixedMargin, LinearLayout},
    object_chain::Chain,
};
use profont::PROFONT_10_POINT;
use strum::EnumString;
pub use weact_studio_epd::TriColor;

use crate::{Widget, forecast::Forecast};

const ROW_MARGIN: i32 = 10;

#[derive(Copy, Clone, Debug, PartialEq, Sequence, EnumString, strum::Display)]
#[cfg_attr(target_os = "none", derive(defmt::Format))]
pub enum Day {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl Day {
    pub fn as_str(&self) -> &'static str {
        match self {
            Day::Sunday => "Sunday",
            Day::Monday => "Monday",
            Day::Tuesday => "Tuesday",
            Day::Wednesday => "Wednesday",
            Day::Thursday => "Thursday",
            Day::Friday => "Friday",
            Day::Saturday => "Saturday",
        }
    }

    pub fn as_short_str(&self) -> &'static str {
        match self {
            Day::Sunday => "Sun",
            Day::Monday => "Mon",
            Day::Tuesday => "Tue",
            Day::Wednesday => "Wed",
            Day::Thursday => "Thu",
            Day::Friday => "Fri",
            Day::Saturday => "Sat",
        }
    }
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
pub fn week_widget(forecasts: &[Forecast; 6]) -> impl Widget {
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
