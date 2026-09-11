# ESP32-S3 reference firmware

Play NES ROMs on an ESP32-S3 under Embassy (`esp-rtos`), using the
hardware-neutral [`nes-embassy`](../..) driver. This is the board-specific
half of the split: everything reusable lives in `nes-embassy`, everything
with a pin number lives here.

## Wiring (reference map, edit `src/board.rs` to retarget)

| Signal   | GPIO | Notes                              |
|----------|------|------------------------------------|
| LCD_SCK  | 12   | SPI2 clock                         |
| LCD_MOSI | 11   | SPI2 data out                      |
| LCD_CS   | 10   | chip select (managed by driver)     |
| LCD_DC   | 9    | data/command                       |
| LCD_RST  | 14   | reset                              |
| LCD_BL   | 15   | backlight (driven high)            |
| BTN_A    | 4    | active-low, internal pull-up         |
| BTN_B    | 5    | active-low, internal pull-up         |
| BTN_SELECT | 6  | active-low, internal pull-up         |
| BTN_START | 7   | active-low, internal pull-up         |
| BTN_UP   | 16   | active-low, internal pull-up         |
| BTN_DOWN | 17   | active-low, internal pull-up         |
| BTN_LEFT | 18   | active-low, internal pull-up         |
| BTN_RIGHT | 8   | active-low, internal pull-up         |
| I2S_BCLK | 1    | I2S0 bit clock                     |
| I2S_WS   | 2    | I2S0 word select                   |
| I2S_DOUT | 3    | I2S0 data to DAC                   |

Display default: ST7789 240x240. The 256-wide NES frame is center-cropped
by 8 px on each side (the overscan area); larger displays letterbox instead.
Swap the model in `src/display.rs` for ILI9341 320x240.

## Toolchain

You need the esp-rs Xtensa toolchain (install via `espup`) new enough for
`esp-rtos 0.4` (Rust >= 1.95):

```sh
espup install
# ...follow espup's instructions to export the toolchain...
cd crates/nes-embassy/firmware/esp32-s3
cargo check
cargo build --release
espflash flash --monitor target/xtensa-esp32s3-none-elf/release/nes-esp32-s3
```

`cargo check`/`cargo build` cannot run on the host target: this crate only
builds for `xtensa-esp32s3-none-elf` (see `.cargo/config.toml`).

## ROM

`src/rom.rs` embeds `roms/homebrew/homebrew.nes` from the repo root via
`include_bytes!`. Drop your own `.nes` next to it and update the path.

## Features

- `psram`: also register the octal PSRAM region with the global allocator.
  Required for larger banked games; the on-chip heap (~320 KiB) fits NROM.
- `qemu`: skip the ST7789 SPI transfer and the I2S DMA transfer. For CI
  only — never on real hardware. Espressif's QEMU fork models no panel and
  no audio codec, so the transfers are stubbed; peripheral *init*, ROM
  loading, and the real emulation loop all still run. See "CI" below.

## CI

`.github/workflows/esp32s3-qemu.yml` boots this firmware in Espressif's
QEMU fork (`esp32s3` machine) on every push/PR touching `nes-core`,
`nes-embassy`, or the workflow itself:

1. Host gates: `cargo fmt --check`, `nes-core` tests, `nes-embassy` tests.
2. Install the Espressif Rust fork via `esp-rs/xtensa-toolchain`
   (the CI route espup's own docs recommend).
3. `cargo +esp build --release --features qemu`.
4. `espflash save-image --chip esp32s3 --merge` to a 4 MB flash image.
5. Boot it with `qemu-system-xtensa -machine esp32s3`, capturing UART0
   (`-serial file:`). The firmware logs `nes: boot`, `nes: rom loaded`,
   `nes: entering frame loop`, and `nes: frame N` every 60 frames over
   `esp-println`'s `uart` transport; CI asserts all four markers plus frame
   60, and fails on any panic/exception signature or an early QEMU exit.

To reproduce the QEMU half locally:

```sh
# after building with --features qemu (see Toolchain above)
espflash save-image --chip esp32s3 --merge --flash-size 4MB \
  target/xtensa-esp32s3-none-elf/release/nes-esp32-s3 /tmp/qemu_flash.bin
qemu-system-xtensa -nographic -machine esp32s3 \
  -drive file=/tmp/qemu_flash.bin,if=mtd,format=raw \
  -monitor none -serial file:/tmp/qemu_serial.log \
  -global driver=timer.esp32c3.timg,property=wdt_disable,value=true
# then: grep "nes: frame" /tmp/qemu_serial.log
```

Get `qemu-system-xtensa` from the
[Espressif QEMU fork releases](https://github.com/espressif/qemu/releases)
(`qemu-xtensa-softmmu-*-x86_64-linux-gnu.tar.xz`).

## Performance notes (honest)

At 240 MHz the S3 has ~4M cycles per 60 Hz frame; the core needs ~6.5M host
cycles per frame, so full-speed 60 fps is unlikely on this chip — expect
roughly half speed on simple games. The ESP32-P4 is the easier target for
full speed. The two biggest firmware-side wins, in order:

1. SPI display writes are currently blocking (~24 ms for a full frame at
   40 MHz). Move them to DMA SPI.
2. The audio path already uses I2S DMA; keep it fed or underruns click.
