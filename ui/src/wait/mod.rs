use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::{Point, Size},
    image::Image,
    mono_font::MonoTextStyle,
    primitives::{Primitive, PrimitiveStyle, RoundedRectangle},
    text::Text,
};
use embedded_iconoir::{icons::size96px::*, prelude::IconoirNewIcon};
use embedded_layout::{
    View,
    align::{Align, horizontal, vertical},
    layout::linear::LinearLayout,
    object_chain::Chain,
};
use profont::PROFONT_24_POINT;
use weact_studio_epd::TriColor;

pub fn draw_message<D>(display: &mut D, round: u8) -> Result<(), D::Error>
where
    D: DrawTarget<Color = TriColor>,
{
    display.clear(TriColor::White)?;

    let (primary, secondary) = if round % 2 == 0 {
        (TriColor::Black, TriColor::Red)
    } else {
        (TriColor::Red, TriColor::Black)
    };

    let text_style = MonoTextStyle::new(&PROFONT_24_POINT, primary);
    let line1 = Text::new("No connection to Hub", Point::zero(), text_style);
    let line2 = Text::new("Check your walls", Point::zero(), text_style);

    let icon = actions::WarningTriangle::new(secondary);
    let icon = Image::new(&icon, Point::zero());

    let content = LinearLayout::vertical(Chain::new(line1).append(icon).append(line2))
        .with_alignment(horizontal::Center)
        .arrange()
        .translate(Point::zero());

    let frame = RoundedRectangle::with_equal_corners(content.bounds().offset(6), Size::new(6, 6))
        .into_styled(PrimitiveStyle::with_stroke(secondary, 3));

    Chain::new(frame)
        .append(content)
        .align_to(
            &display.bounding_box(),
            horizontal::Center,
            vertical::Center,
        )
        .draw(display)?;

    Ok(())
}
