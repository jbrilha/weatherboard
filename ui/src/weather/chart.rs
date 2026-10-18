use core::fmt::Write;

use embedded_graphics::{
    geometry::Point,
    mono_font::MonoTextStyle,
    primitives::{Polyline, Primitive, PrimitiveStyle},
    text::{Alignment, Text},
};
use embedded_layout::{
    View,
    align::{horizontal, vertical},
    layout::linear::{FixedMargin, LinearLayout},
    object_chain::Chain,
};
use heapless::String;
use profont::{PROFONT_7_POINT, PROFONT_9_POINT};
use weact_studio_epd::TriColor;

use crate::Widget;

const X_LINE_MIN: i32 = 30;
const X_LINE_STEP: i32 = 31;
const Y_LINE_TOP: i32 = 160;
const Y_LINE_BOT: i32 = 190;

const HIGH_LOW_SPACING: i32 = 20;

const PCT_100: &str = "100%";
const PCT_0: &str = "0%";

const fn hour_x(i: i32) -> i32 {
    X_LINE_MIN + i * X_LINE_STEP
}

const TEMP_POINTS: [Point; 12] = [
    Point::new(hour_x(0), Y_LINE_BOT),
    Point::new(hour_x(1), Y_LINE_TOP),
    Point::new(hour_x(2), Y_LINE_BOT),
    Point::new(hour_x(3), Y_LINE_TOP),
    Point::new(hour_x(4), Y_LINE_BOT),
    Point::new(hour_x(5), Y_LINE_TOP),
    Point::new(hour_x(6), Y_LINE_BOT),
    Point::new(hour_x(7), Y_LINE_TOP),
    Point::new(hour_x(8), Y_LINE_BOT),
    Point::new(hour_x(9), Y_LINE_TOP),
    Point::new(hour_x(10), Y_LINE_BOT),
    Point::new(hour_x(11), Y_LINE_TOP),
];

const POP_POINTS: [Point; 12] = [
    Point::new(hour_x(0), Y_LINE_TOP),
    Point::new(hour_x(1), Y_LINE_BOT),
    Point::new(hour_x(2), Y_LINE_TOP),
    Point::new(hour_x(3), Y_LINE_BOT),
    Point::new(hour_x(4), Y_LINE_TOP),
    Point::new(hour_x(5), Y_LINE_BOT),
    Point::new(hour_x(6), Y_LINE_TOP),
    Point::new(hour_x(7), Y_LINE_BOT),
    Point::new(hour_x(8), Y_LINE_TOP),
    Point::new(hour_x(9), Y_LINE_BOT),
    Point::new(hour_x(10), Y_LINE_TOP),
    Point::new(hour_x(11), Y_LINE_BOT),
];

pub struct Chart {
    pub temp_high_txt: String<4>,
    pub temp_low_txt: String<4>,
    pub hour_txt: [String<4>; 12],
}

impl Chart {
    pub fn new(temp_low: i8, temp_high: i8, hours: [u8; 12]) -> Self {
        let mut temp_high_txt = String::new();
        write!(temp_high_txt, "{temp_high}°").unwrap();

        let mut temp_low_txt = String::new();
        write!(temp_low_txt, "{temp_low}°").unwrap();

        Self {
            temp_high_txt,
            temp_low_txt,
            hour_txt: hours.map(hour_label),
        }
    }
}

fn hour_label(hour: u8) -> String<4> {
    let suffix = if hour % 24 < 12 { "AM" } else { "PM" };
    let h12 = match hour % 12 {
        0 => 12,
        h => h,
    };

    let mut txt = String::new();
    write!(txt, "{h12}{suffix}").unwrap();
    txt
}

pub fn chart_widget(chart: &Chart) -> impl Widget {
    let temp_style = MonoTextStyle::new(&PROFONT_9_POINT, TriColor::Red);
    let high_temp = Text::new(&chart.temp_high_txt, Point::zero(), temp_style);
    let low_temp = Text::new(&chart.temp_low_txt, Point::zero(), temp_style);

    let pop_style = MonoTextStyle::new(&PROFONT_9_POINT, TriColor::Black);
    let high_pop = Text::new(PCT_100, Point::zero(), pop_style);
    let low_pop = Text::new(PCT_0, Point::zero(), pop_style);

    let temp = LinearLayout::vertical(Chain::new(high_temp).append(low_temp))
        .with_alignment(horizontal::Center)
        .with_spacing(FixedMargin(HIGH_LOW_SPACING))
        .arrange();

    let pop = LinearLayout::vertical(Chain::new(high_pop).append(low_pop))
        .with_alignment(horizontal::Center)
        .with_spacing(FixedMargin(HIGH_LOW_SPACING))
        .arrange();

    let lines = Chain::new(chart_line(&TEMP_POINTS, TriColor::Red))
        .append(chart_line(&POP_POINTS, TriColor::Black));
    let lines_origin = lines.bounds().top_left;

    let row = LinearLayout::horizontal(Chain::new(temp).append(lines).append(pop))
        .with_alignment(vertical::Center)
        .with_spacing(FixedMargin(4))
        .arrange();

    let lines_offset = row.inner().parent.object.bounds().top_left - lines_origin;

    Chain::new(row).append(hour_labels(chart).translate(lines_offset))
}

fn chart_line(points: &[Point; 12], color: TriColor) -> impl Widget {
    let line_style = PrimitiveStyle::with_stroke(color, 2);

    Polyline::new(points).into_styled(line_style)
}

fn hour_labels(chart: &Chart) -> impl Widget {
    let style = MonoTextStyle::new(&PROFONT_7_POINT, TriColor::Black);
    let label = |i: usize| {
        Text::with_alignment(
            &chart.hour_txt[i],
            Point::new(hour_x(i as i32), Y_LINE_BOT + 12),
            style,
            Alignment::Center,
        )
    };

    Chain::new(label(0))
        .append(label(1))
        .append(label(2))
        .append(label(3))
        .append(label(4))
        .append(label(5))
        .append(label(6))
        .append(label(7))
        .append(label(8))
        .append(label(9))
        .append(label(10))
        .append(label(11))
}
