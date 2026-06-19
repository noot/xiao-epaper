#![no_std]
#![no_main]

use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{
    Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StrokeAlignment, Triangle,
};
use embedded_graphics::text::Text;
use esp_println::println;

use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::main;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::time::Rate;

mod uc8179;
use uc8179::{FB_SIZE, Uc8179};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

static mut FRAMEBUFFER: [u8; FB_SIZE] = [0u8; FB_SIZE];

#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    // xiao 7.5" epaper panel SPI pins: SCK=GPIO8, MOSI=GPIO10
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(10))
            .with_mode(Mode::_0),
    )
    .expect("static spi config is valid")
    .with_sck(peripherals.GPIO8)
    .with_mosi(peripherals.GPIO10);

    // xiao 7.5" epaper panel control pins
    let cs = Output::new(peripherals.GPIO3, Level::High, OutputConfig::default());
    let dc = Output::new(peripherals.GPIO5, Level::Low, OutputConfig::default());
    let rst = Output::new(peripherals.GPIO2, Level::High, OutputConfig::default());
    let busy = Input::new(
        peripherals.GPIO4,
        InputConfig::default().with_pull(Pull::None),
    );
    let delay = Delay::new();

    // safety: taken exactly once at startup; no other reference to FRAMEBUFFER exists.
    let fb_ptr: *mut [u8; FB_SIZE] = &raw mut FRAMEBUFFER;
    let fb: &'static mut [u8; FB_SIZE] = unsafe { &mut *fb_ptr };

    let mut display = Uc8179::new(spi, cs, dc, rst, busy, delay, fb);

    println!("xiao-epaper: starting init");
    match display.init() {
        Ok(()) => println!("xiao-epaper: init ok"),
        Err(e) => println!("xiao-epaper: init failed: {:?}", e),
    }

    println!("xiao-epaper: drawing to framebuffer");
    display.clear_white();
    let _ = draw_demo(&mut display);

    println!("xiao-epaper: flushing to display");
    match display.flush() {
        Ok(()) => println!("xiao-epaper: flush ok, display should refresh now"),
        Err(e) => println!("xiao-epaper: flush failed: {:?}", e),
    }

    loop {
        core::hint::spin_loop();
    }
}

fn draw_demo<D>(d: &mut D) -> Result<(), D::Error>
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
