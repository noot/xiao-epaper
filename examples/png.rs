#![no_std]
#![no_main]

use esp_println::println;

use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_hal::main;
use esp_hal::spi::Mode;
use esp_hal::spi::master::{Config as SpiConfig, Spi};
use esp_hal::time::Rate;

use xiao_epaper::uc8179::{FB_SIZE, Uc8179};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

static IMAGE: &[u8; FB_SIZE] = include_bytes!(concat!(env!("OUT_DIR"), "/image.raw"));
static mut FRAMEBUFFER: [u8; FB_SIZE] = [0u8; FB_SIZE];

#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(10))
            .with_mode(Mode::_0),
    )
    .expect("static spi config is valid")
    .with_sck(peripherals.GPIO8)
    .with_mosi(peripherals.GPIO10);

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

    println!("xiao-epaper: loading image");
    display.load_raw(IMAGE);

    println!("xiao-epaper: flushing to display");
    match display.flush() {
        Ok(()) => println!("xiao-epaper: flush ok, display should refresh now"),
        Err(e) => println!("xiao-epaper: flush failed: {:?}", e),
    }

    loop {
        core::hint::spin_loop();
    }
}
