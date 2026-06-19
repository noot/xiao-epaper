# xiao-epaper

Rust `no_std` firmware for the [Seeed Studio XIAO 7.5" ePaper Panel](https://www.seeedstudio.com/XIAO-7-5-ePaper-Panel-p-6416.html).

UC8179 display driver and example firmwares for the 800x480 e-ink display, using `esp-hal` and `embedded-graphics`.

## Hardware

- **MCU:** ESP32-C3 (RISC-V, 160 MHz) via the XIAO ESP32-C3 module
- **Display:** 7.5" e-ink, 800x480, UC8179 (GD7965) controller
- **Interface:** SPI (SCK=GPIO8, MOSI=GPIO10, CS=GPIO3, DC=GPIO5, RST=GPIO2, BUSY=GPIO4)

## Prerequisites

Install the Rust toolchain, `espflash`, and `just`:

```sh
rustup target add riscv32imc-unknown-none-elf
cargo install espflash
cargo install just
```

## Setup

```sh
cp .env.example .env
# edit .env with your wifi credentials and server IP
```

## Examples

Connect the XIAO panel via USB-C. All examples flash the firmware and open a serial monitor.

### demo

Draws text, shapes, and a checkerboard pattern.

```sh
just demo
```

### png

Renders a PNG image from `assets/image.png`. The build script converts it to 1bpp with Floyd-Steinberg dithering at compile time.

```sh
just png
```

### fetch

Connects to wifi, fetches a pre-rendered framebuffer from the image server, and displays it. Partial refresh every minute, full refresh every 5 minutes to clear ghosting. Requires `SSID`, `PASSWORD`, and `SERVER_URL` in `.env`.

```sh
just server   # in one terminal
just fetch    # in another
```

## Image Server

A standalone axum server that converts a PNG to a 1bpp framebuffer and serves it over HTTP.

```sh
just server                        # serves assets/image.png
just server ~/Pictures/photo.png   # serves a specific image
```

Listens on `0.0.0.0:3000` by default. Set `PORT` in `.env` to change it (and update `SERVER_URL` to match). The firmware fetches from `GET /framebuffer`.

## Troubleshooting

- **Device not found:** make sure the USB-C cable supports data (not charge-only). Try holding the BOOT button while plugging in to enter bootloader mode.
- **Display doesn't refresh:** the UC8179 init sequence may need adjustment. Check the busy pin timeout in `src/uc8179.rs`.
