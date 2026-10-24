#![no_std]

use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    mono_font::MonoTextStyle,
    primitives::{Circle, Line, Primitive, PrimitiveStyle, Rectangle},
    text::Text,
};
use embedded_layout::View;
use profont::PROFONT_14_POINT;
pub use weact_studio_epd::TriColor;

#[macro_use]
mod fmt;

pub mod wait;
pub mod weather;

pub trait Widget: View + Drawable<Color = TriColor> {}
impl<T: View + Drawable<Color = TriColor>> Widget for T {}

pub fn test_pattern<D>(display: &mut D, label: &str, round: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    display.clear(TriColor::White)?;

    let (primary, secondary) = if round % 2 == 0 {
        (TriColor::Black, TriColor::Red)
    } else {
        (TriColor::Red, TriColor::Black)
    };

    Text::new(
        label,
        Point::new(10, 24),
        MonoTextStyle::new(&PROFONT_14_POINT, primary),
    )
    .draw(display)?;
    Text::new(
        "black red white",
        Point::new(10, 48),
        MonoTextStyle::new(&PROFONT_14_POINT, secondary),
    )
    .draw(display)?;

    let swatches = [TriColor::Black, TriColor::Red, TriColor::White];
    for (i, color) in swatches.into_iter().enumerate() {
        let top_left = Point::new(10 + i as i32 * 130, 64);
        Rectangle::new(top_left, Size::new(120, 90))
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(display)?;
        Rectangle::new(top_left, Size::new(120, 90))
            .into_styled(PrimitiveStyle::with_stroke(TriColor::Black, 1))
            .draw(display)?;
    }

    for i in 0..40 {
        let x = 10 + i * 4;
        let color = if i % 2 == 0 { primary } else { secondary };
        Line::new(Point::new(x, 170), Point::new(x, 290))
            .into_styled(PrimitiveStyle::with_stroke(color, 1))
            .draw(display)?;
    }

    Circle::new(Point::new(190, 170), 120)
        .into_styled(PrimitiveStyle::with_fill(secondary))
        .draw(display)?;
    Circle::new(Point::new(220, 200), 60)
        .into_styled(PrimitiveStyle::with_fill(primary))
        .draw(display)?;

    Rectangle::new(Point::new(320, 170), Size::new(65, 120))
        .into_styled(PrimitiveStyle::with_stroke(secondary, 3))
        .draw(display)?;

    Ok(())
}
