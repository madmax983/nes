//! Build script for the ESP32-S3 NES firmware.
//!
//! Tells the linker to use esp-hal's `linkall.x` linker script, which includes
//! the ESP32-S3-specific memory layout, stack definitions, and section
//! placements. This is required for a correct ELF; the generic
//! `xtensa-lx-rt` script produces invalid flash addresses.

fn main() {
    println!("cargo:rustc-link-arg=-Tlinkall.x");
    println!("cargo:rerun-if-changed=build.rs");
}
