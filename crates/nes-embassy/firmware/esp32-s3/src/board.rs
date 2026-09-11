//! Reference pin map for the ESP32-S3 NES handheld.
//!
//! This is the only file you need to edit to retarget the firmware to your
//! own board: every pin below is a plain GPIO number, and [`steal_pin`]
//! turns those numbers into the type-erased pins the drivers take.

use esp_hal::gpio::AnyPin;

/// Takes GPIO `n` as a type-erased pin.
///
/// This goes around the typed `peripherals.GPIOx` singletons so the whole
/// wiring map can live in this file as plain numbers. It is sound because
/// each number is stolen at most once and the corresponding typed singleton
/// is never used.
///
/// # Panics
///
/// Panics at boot when `n` is not a valid ESP32-S3 GPIO. That is a wiring
/// bug in this file, so failing loudly here is correct. (`AnyPin::steal`
/// itself panics on unknown numbers.)
#[must_use]
pub fn steal_pin(n: u8) -> AnyPin<'static> {
    // SAFETY: single-ownership contract documented above; each board constant
    // is stolen exactly once in `main`.
    unsafe { AnyPin::steal(n) }
}

/// SPI2 pins for the ST7789 display.
pub const LCD_SCK: u8 = 12;
pub const LCD_MOSI: u8 = 11;
pub const LCD_CS: u8 = 10;
pub const LCD_DC: u8 = 9;
pub const LCD_RST: u8 = 14;
pub const LCD_BL: u8 = 15;

/// Display geometry. The stock target is a 240x240 ST7789; the 256-wide NES
/// frame is center-cropped to fit (the crop eats overscan, not playfield).
/// Set these to 320x240 for an ILI9341 and the frame letterboxes instead.
pub const DISP_W: u16 = 240;
pub const DISP_H: u16 = 240;
/// Row offset some 240x240 ST7789 variants need (set to 0 if yours differs).
pub const DISP_Y_OFFSET: u16 = 80;

/// Controller buttons, active-low with internal pull-ups.
pub const BTN_A: u8 = 4;
pub const BTN_B: u8 = 5;
pub const BTN_SELECT: u8 = 6;
pub const BTN_START: u8 = 7;
pub const BTN_UP: u8 = 16;
pub const BTN_DOWN: u8 = 17;
pub const BTN_LEFT: u8 = 18;
pub const BTN_RIGHT: u8 = 8;

/// I2S0 pins to the DAC (44.1 kHz stereo, 16-bit Philips format).
pub const I2S_BCLK: u8 = 1;
pub const I2S_WS: u8 = 2;
pub const I2S_DOUT: u8 = 3;

/// SPI clock for the display. 40 MHz is safe for most ST7789 modules.
pub const LCD_SPI_HZ: u32 = 40_000_000;

/// Heap for the emulator + Embassy. NROM-class games fit comfortably;
/// enable the `psram` feature for large banked games.
///
/// QEMU builds use a smaller heap (200 KiB) to fit within the ESP32-S3's
/// 414 KiB of usable DRAM alongside the static allocations. The EmuDriver
/// needs ~200 KiB (77K NesCore + 123K frame + 1.5K audio).
#[cfg(feature = "qemu")]
pub const HEAP_BYTES: usize = 200 * 1024;
/// Heap for real hardware (320 KiB).
#[cfg(not(feature = "qemu"))]
pub const HEAP_BYTES: usize = 320 * 1024;
