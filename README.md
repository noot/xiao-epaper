# xiao-epaper

Rust `no_std` firmware for the [Seeed Studio XIAO 7.5" ePaper Panel](https://www.seeedstudio.com/XIAO-7-5-ePaper-Panel-p-6416.html).

Draws a demo image (text, shapes, checkerboard) on the 800x480 e-ink display using `esp-hal` and `embedded-graphics`.

## Hardware

- **MCU:** ESP32-C3 (RISC-V, 160 MHz) via the XIAO ESP32-C3 module
- **Display:** 7.5" e-ink, 800x480, UC8179 (GD7965) controller
- **Interface:** SPI (SCK=GPIO8, MOSI=GPIO10, CS=GPIO3, DC=GPIO5, RST=GPIO2, BUSY=GPIO4)

## Prerequisites

Install the Rust toolchain and `espflash`:

```sh
rustup target add riscv32imc-unknown-none-elf
cargo install espflash
```

## Build

```sh
cargo build --release
```

## Flash

Connect the XIAO panel via USB-C, then:

```sh
cargo run --release
```

This flashes the firmware and opens a serial monitor. If the device isn't detected automatically, specify the port:

```sh
espflash flash --monitor --chip esp32c3 target/riscv32imc-unknown-none-elf/release/xiao-epaper --port /dev/ttyACM0
```

The display should refresh after a few seconds with the demo image.

## Troubleshooting

- **Device not found:** make sure the USB-C cable supports data (not charge-only). Try holding the BOOT button while plugging in to enter bootloader mode.
- **Display doesn't refresh:** the UC8179 init sequence may need adjustment. Check the busy pin timeout in `src/uc8179.rs`.
