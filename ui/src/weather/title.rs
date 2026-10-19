use common::weather::{Location, Month};
use embedded_graphics::{
    geometry::Point,
    mono_font::MonoTextStyle,
    primitives::{Line, Primitive, PrimitiveStyle},
    text::Text,
};
use embedded_layout::{
    align::vertical,
    layout::linear::{FixedMargin, LinearLayout},
    object_chain::Chain,
};
use heapless::String;
use profont::PROFONT_14_POINT;
use weact_studio_epd::TriColor;

use crate::Widget;
use crate::weather::{ROW_MARGIN, day::Day};
use core::fmt::Write;

pub struct Title {
    pub location: String<16>,
    pub week_day: Day,
    pub date: String<6>,
    pub updated: String<5>,
}
impl Title {
    pub fn new(
        location: Location,
        week_day: Day,
        month: Month,
        day: u8,
        (hours, minutes): (u8, u8),
    ) -> Self {
        let mut date: String<6> = String::new();
        write!(date, "{} {}", month.as_short_str(), day).unwrap();

        let mut updated: String<5> = String::new();
        write!(updated, "{hours:02}:{minutes:02}").unwrap();

        Self {
            location: String::try_from(location.name()).unwrap(),
            week_day,
            date,
            updated,
        }
    }
}

pub fn title_row(title: &Title) -> impl Widget {
    let style = MonoTextStyle::new(&PROFONT_14_POINT, TriColor::Red);

    let divider = Line::new(Point::zero(), Point::new(0, 14))
        .into_styled(PrimitiveStyle::with_stroke(TriColor::Red, 1));

    LinearLayout::horizontal(
        Chain::new(Text::new(title.location.as_str(), Point::zero(), style))
            .append(divider)
            .append(Text::new(title.week_day.as_str(), Point::zero(), style))
            .append(Text::new(title.date.as_str(), Point::zero(), style))
            .append(divider)
            .append(Text::new(title.updated.as_str(), Point::zero(), style)),
    )
    .with_alignment(vertical::Center)
    .with_spacing(FixedMargin(ROW_MARGIN))
    .arrange()
}
