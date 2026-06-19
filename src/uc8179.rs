//! minimal UC8179 (GD7965) driver for the seeed studio XIAO 7.5" ePaper panel.
//!
//! command sequence ported from bb_epaper (EP75_800x480) as used in the
//! Seeed TRMNL firmware for this exact board.

use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal::spi::SpiBus;
use esp_println::println;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 480;
const ROW_BYTES: usize = (WIDTH as usize) / 8;
pub const FB_SIZE: usize = ROW_BYTES * HEIGHT as usize;

const CMD_PANEL_SETTING: u8 = 0x00;
const CMD_POWER_SETTING: u8 = 0x01;
const CMD_POWER_ON: u8 = 0x04;
const CMD_DISPLAY_REFRESH: u8 = 0x12;
const CMD_WRITE_RAM_NEW: u8 = 0x13;
const CMD_VCOM_INTERVAL: u8 = 0x50;
const CMD_TCON: u8 = 0x60;
const CMD_RESOLUTION: u8 = 0x61;
const CMD_LUT_VCOM: u8 = 0x20;
const CMD_LUT_WW: u8 = 0x21;
const CMD_LUT_BW: u8 = 0x22;
const CMD_LUT_WB: u8 = 0x23;
const CMD_LUT_BB: u8 = 0x24;
const CMD_LUT_BD: u8 = 0x25;
const CMD_PARTIAL_WINDOW: u8 = 0x90;
const CMD_VCOM_DC: u8 = 0x82;

// partial refresh LUTs from GxEPD2 (GDEW075T7).
// format: [waveform, T1, T2, T3, T4, repeat, ...zeros to 42 bytes]
// T1=30, T2=5, T3=30, T4=5 — two drive phases with short pauses.
#[rustfmt::skip]
const LUT_PARTIAL_VCOM: [u8; 42] = [
    0x00, 30, 5, 30, 5, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
#[rustfmt::skip]
const LUT_PARTIAL_WW: [u8; 42] = [
    0x00, 30, 5, 30, 5, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
#[rustfmt::skip]
const LUT_PARTIAL_BW: [u8; 42] = [
    0x5A, 30, 5, 30, 5, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
#[rustfmt::skip]
const LUT_PARTIAL_WB: [u8; 42] = [
    0x84, 30, 5, 30, 5, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
#[rustfmt::skip]
const LUT_PARTIAL_BB: [u8; 42] = [
    0x00, 30, 5, 30, 5, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];
#[rustfmt::skip]
const LUT_PARTIAL_BD: [u8; 42] = [
    0x00, 30, 5, 30, 5, 1, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[derive(Debug)]
pub enum Error<S, P> {
    Spi(S),
    Pin(P),
}

pub struct Uc8179<'a, SPI, CS, DC, RST, BUSY, DELAY> {
    spi: SPI,
    cs: CS,
    dc: DC,
    rst: RST,
    busy: BUSY,
    delay: DELAY,
    fb: &'a mut [u8; FB_SIZE],
}

impl<'a, SPI, CS, DC, RST, BUSY, DELAY> Uc8179<'a, SPI, CS, DC, RST, BUSY, DELAY>
where
    SPI: SpiBus,
    CS: OutputPin,
    DC: OutputPin<Error = CS::Error>,
    RST: OutputPin<Error = CS::Error>,
    BUSY: InputPin<Error = CS::Error>,
    DELAY: DelayNs,
{
    pub fn new(
        spi: SPI,
        cs: CS,
        dc: DC,
        rst: RST,
        busy: BUSY,
        delay: DELAY,
        fb: &'a mut [u8; FB_SIZE],
    ) -> Self {
        Self {
            spi,
            cs,
            dc,
            rst,
            busy,
            delay,
            fb,
        }
    }

    fn cmd(&mut self, c: u8) -> Result<(), Error<SPI::Error, CS::Error>> {
        self.dc.set_low().map_err(Error::Pin)?;
        self.cs.set_low().map_err(Error::Pin)?;
        self.spi.write(&[c]).map_err(Error::Spi)?;
        self.spi.flush().map_err(Error::Spi)?;
        self.cs.set_high().map_err(Error::Pin)?;
        Ok(())
    }

    fn data(&mut self, d: &[u8]) -> Result<(), Error<SPI::Error, CS::Error>> {
        self.dc.set_high().map_err(Error::Pin)?;
        self.cs.set_low().map_err(Error::Pin)?;
        self.spi.write(d).map_err(Error::Spi)?;
        self.spi.flush().map_err(Error::Spi)?;
        self.cs.set_high().map_err(Error::Pin)?;
        Ok(())
    }

    fn cmd_with(&mut self, c: u8, d: &[u8]) -> Result<(), Error<SPI::Error, CS::Error>> {
        self.cmd(c)?;
        self.data(d)
    }

    // UC81xx busy: LOW = busy, HIGH = ready (idle).
    fn wait_busy(&mut self, label: &str, timeout_ms: u32) {
        self.delay.delay_ms(10);
        let mut elapsed = 10u32;

        while self.busy.is_low().unwrap_or(true) {
            self.delay.delay_ms(100);
            elapsed += 100;
            if elapsed >= timeout_ms {
                println!("  busy ({}): timeout after {}ms", label, elapsed);
                return;
            }
        }
        println!("  busy ({}): ready after {}ms", label, elapsed);
    }

    fn reset(&mut self) -> Result<(), Error<SPI::Error, CS::Error>> {
        self.rst.set_low().map_err(Error::Pin)?;
        self.delay.delay_ms(10);
        self.rst.set_high().map_err(Error::Pin)?;
        self.delay.delay_ms(20);
        Ok(())
    }

    // bb_epaper epd75_init_sequence_full: sent before every refresh.
    //   PWR → PON → BUSY_WAIT → PSR → TRES → 0x15 → CDI → TCON
    fn send_init_sequence(&mut self) -> Result<(), Error<SPI::Error, CS::Error>> {
        self.cmd_with(CMD_POWER_SETTING, &[0x07, 0x07, 0x3f, 0x3f])?;
        self.cmd(CMD_POWER_ON)?;
        self.wait_busy("power-on", 10000);
        self.cmd_with(CMD_PANEL_SETTING, &[0x1f])?;
        self.cmd_with(CMD_RESOLUTION, &[0x03, 0x20, 0x01, 0xE0])?;
        self.cmd_with(0x15, &[0x00])?;
        self.cmd_with(CMD_VCOM_INTERVAL, &[0x29, 0x07])?;
        self.cmd_with(CMD_TCON, &[0x22])?;
        Ok(())
    }

    pub fn init(&mut self) -> Result<(), Error<SPI::Error, CS::Error>> {
        println!("uc8179: hardware reset");
        self.reset()?;
        self.wait_busy("reset", 5000);
        println!("uc8179: init done");
        Ok(())
    }

    pub fn clear_white(&mut self) {
        self.fb.fill(0xFF);
    }

    pub fn load_raw(&mut self, data: &[u8; FB_SIZE]) {
        self.fb.copy_from_slice(data);
    }

    pub fn framebuffer_mut(&mut self) -> &mut [u8; FB_SIZE] {
        self.fb
    }

    pub fn flush(&mut self) -> Result<(), Error<SPI::Error, CS::Error>> {
        println!("uc8179: full flush start");

        self.write_framebuffer()?;
        self.send_init_sequence()?;

        self.cmd(CMD_DISPLAY_REFRESH)?;
        self.wait_busy("refresh", 30000);
        println!("uc8179: full flush done");
        Ok(())
    }

    // uses register LUTs with a partial waveform — only changed pixels
    // are driven. may ghost over time; use flush() periodically.
    pub fn flush_partial(&mut self) -> Result<(), Error<SPI::Error, CS::Error>> {
        println!("uc8179: partial flush start");

        self.send_init_sequence()?;

        // override to register LUT mode (bit 5 = 1)
        self.cmd_with(CMD_PANEL_SETTING, &[0x3f])?;
        self.cmd_with(CMD_VCOM_DC, &[0x26])?;
        // N2OCP=1 (bit 4) for new→old RAM copy, BDV=11 for LUTBD border
        self.cmd_with(CMD_VCOM_INTERVAL, &[0x39, 0x07])?;

        self.cmd_with(CMD_LUT_VCOM, &LUT_PARTIAL_VCOM)?;
        self.cmd_with(CMD_LUT_WW, &LUT_PARTIAL_WW)?;
        self.cmd_with(CMD_LUT_BW, &LUT_PARTIAL_BW)?;
        self.cmd_with(CMD_LUT_WB, &LUT_PARTIAL_WB)?;
        self.cmd_with(CMD_LUT_BB, &LUT_PARTIAL_BB)?;
        self.cmd_with(CMD_LUT_BD, &LUT_PARTIAL_BD)?;

        self.write_framebuffer()?;

        #[rustfmt::skip]
        self.cmd_with(CMD_PARTIAL_WINDOW, &[
            0x00, 0x00,       // x start
            0x03, 0x1F,       // x end (799)
            0x00, 0x00,       // y start
            0x01, 0xDF,       // y end (479)
            0x01,             // scan inside partial area only
        ])?;

        self.cmd(CMD_DISPLAY_REFRESH)?;
        self.wait_busy("refresh", 30000);
        println!("uc8179: partial flush done");
        Ok(())
    }

    fn write_framebuffer(&mut self) -> Result<(), Error<SPI::Error, CS::Error>> {
        self.cmd(CMD_WRITE_RAM_NEW)?;
        self.dc.set_high().map_err(Error::Pin)?;
        self.cs.set_low().map_err(Error::Pin)?;
        for chunk in self.fb.chunks(ROW_BYTES) {
            self.spi.write(chunk).map_err(Error::Spi)?;
            self.spi.flush().map_err(Error::Spi)?;
        }
        self.cs.set_high().map_err(Error::Pin)?;
        Ok(())
    }

    fn set_pixel(&mut self, x: u32, y: u32, black: bool) {
        if x >= WIDTH || y >= HEIGHT {
            return;
        }
        let idx = y as usize * ROW_BYTES + (x as usize >> 3);
        let mask = 0x80u8 >> (x & 7);
        if black {
            self.fb[idx] &= !mask;
        } else {
            self.fb[idx] |= mask;
        }
    }
}

impl<SPI, CS, DC, RST, BUSY, DELAY> Dimensions for Uc8179<'_, SPI, CS, DC, RST, BUSY, DELAY> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(Point::zero(), Size::new(WIDTH, HEIGHT))
    }
}

impl<SPI, CS, DC, RST, BUSY, DELAY> DrawTarget for Uc8179<'_, SPI, CS, DC, RST, BUSY, DELAY>
where
    SPI: SpiBus,
    CS: OutputPin,
    DC: OutputPin<Error = CS::Error>,
    RST: OutputPin<Error = CS::Error>,
    BUSY: InputPin<Error = CS::Error>,
    DELAY: DelayNs,
{
    type Color = BinaryColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(point, color) in pixels {
            if point.x >= 0 && point.y >= 0 {
                self.set_pixel(point.x as u32, point.y as u32, color.is_on());
            }
        }
        Ok(())
    }
}
