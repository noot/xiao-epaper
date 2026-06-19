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

pub(crate) const WIDTH: u32 = 800;
pub(crate) const HEIGHT: u32 = 480;
const ROW_BYTES: usize = (WIDTH as usize) / 8;
pub(crate) const FB_SIZE: usize = ROW_BYTES * HEIGHT as usize;

const CMD_PANEL_SETTING: u8 = 0x00;
const CMD_POWER_SETTING: u8 = 0x01;
const CMD_POWER_ON: u8 = 0x04;
const CMD_DISPLAY_REFRESH: u8 = 0x12;
const CMD_WRITE_RAM_NEW: u8 = 0x13;
const CMD_VCOM_INTERVAL: u8 = 0x50;
const CMD_TCON: u8 = 0x60;
const CMD_RESOLUTION: u8 = 0x61;
const CMD_PARTIAL_OUT: u8 = 0x92;

#[derive(Debug)]
pub(crate) enum Error<S, P> {
    Spi(S),
    Pin(P),
}

pub(crate) struct Uc8179<'a, SPI, CS, DC, RST, BUSY, DELAY> {
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
    pub(crate) fn new(
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
        self.cmd_with(CMD_VCOM_INTERVAL, &[0x21, 0x07])?;
        self.cmd_with(CMD_TCON, &[0x22])?;
        Ok(())
    }

    pub(crate) fn init(&mut self) -> Result<(), Error<SPI::Error, CS::Error>> {
        println!("uc8179: hardware reset");
        self.reset()?;
        self.wait_busy("reset", 5000);
        println!("uc8179: init done");
        Ok(())
    }

    pub(crate) fn clear_white(&mut self) {
        self.fb.fill(0xFF);
    }

    pub(crate) fn flush(&mut self) -> Result<(), Error<SPI::Error, CS::Error>> {
        println!("uc8179: flush start");

        self.cmd(CMD_WRITE_RAM_NEW)?;
        self.dc.set_high().map_err(Error::Pin)?;
        self.cs.set_low().map_err(Error::Pin)?;
        for chunk in self.fb.chunks(ROW_BYTES) {
            self.spi.write(chunk).map_err(Error::Spi)?;
            self.spi.flush().map_err(Error::Spi)?;
        }
        self.cs.set_high().map_err(Error::Pin)?;

        self.send_init_sequence()?;

        self.cmd(CMD_PARTIAL_OUT)?;
        self.cmd(CMD_DISPLAY_REFRESH)?;
        self.wait_busy("refresh", 30000);
        println!("uc8179: flush done");
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
