use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
};
use embedded_graphics::text::Text;

pub fn draw<D>(d: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    let ink = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);

    let border = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(3)
        .stroke_alignment(StrokeAlignment::Inside)
        .build();

    let filled = PrimitiveStyle::with_fill(BinaryColor::On);
    let outline = PrimitiveStyleBuilder::new()
        .stroke_color(BinaryColor::On)
        .stroke_width(2)
        .build();

    // outer border
    Rectangle::new(Point::zero(), Size::new(800, 480))
        .into_styled(border)
        .draw(d)?;

    // title
    Text::new("XIAO 7.5\" ePaper Panel", Point::new(40, 50), ink).draw(d)?;
    Text::new("800x480 . UC8179 . ESP32-C3", Point::new(40, 80), ink).draw(d)?;
    Text::new(
        "Rust no_std + esp-hal + embedded-graphics",
        Point::new(40, 110),
        ink,
    )
    .draw(d)?;

    // divider line
    Rectangle::new(Point::new(30, 130), Size::new(740, 2))
        .into_styled(filled)
        .draw(d)?;

    // shapes demo
    Text::new("shapes:", Point::new(40, 170), ink).draw(d)?;

    // filled circle
    Circle::new(Point::new(40, 190), 80)
        .into_styled(filled)
        .draw(d)?;

    // outlined circle
    Circle::new(Point::new(160, 190), 80)
        .into_styled(outline)
        .draw(d)?;

    // filled rectangle
    Rectangle::new(Point::new(280, 190), Size::new(80, 80))
        .into_styled(filled)
        .draw(d)?;

    // outlined rectangle
    Rectangle::new(Point::new(400, 190), Size::new(80, 80))
        .into_styled(outline)
        .draw(d)?;

    // filled triangle
    Triangle::new(
        Point::new(560, 270),
        Point::new(520, 190),
        Point::new(600, 190),
    )
    .into_styled(filled)
    .draw(d)?;

    // outlined triangle
    Triangle::new(
        Point::new(700, 270),
        Point::new(660, 190),
        Point::new(740, 190),
    )
    .into_styled(outline)
    .draw(d)?;

    // checkerboard pattern
    Text::new("pattern:", Point::new(40, 320), ink).draw(d)?;
    let sq = 20u32;
    for row in 0..5 {
        for col in 0..20 {
            if (row + col) % 2 == 0 {
                Rectangle::new(
                    Point::new(40 + (col * sq) as i32, 340 + (row * sq) as i32),
                    Size::new(sq, sq),
                )
                .into_styled(filled)
                .draw(d)?;
            }
        }
    }

    Ok(())
}
