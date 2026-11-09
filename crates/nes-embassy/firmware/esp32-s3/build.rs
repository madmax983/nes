//! Build script for the ESP32-S3 NES firmware.
//!
//! Tells the linker to use esp-hal's `linkall.x` linker script, which includes
//! the ESP32-S3-specific memory layout, stack definitions, and section
//! placements. This is required for a correct ELF; the generic
//! `xtensa-lx-rt` script produces invalid flash addresses.
//!
//! With the `qemu` feature, uses `ld/qemu-linkall.x` instead, which routes
//! ROTEXT->IRAM and RODATA->DRAM so the image has no flash-mapped segments.
//! The ESP32-S3 QEMU machine's ROM loader faults on DROM (Tallow boots in the
//! same QEMU because its image is pure internal RAM).

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    println!("cargo:rustc-link-search={}/ld", manifest_dir);
    if std::env::var("CARGO_FEATURE_QEMU").is_ok() {
        println!("cargo:rustc-link-arg=-Tqemu-linkall.x");
    } else {
        println!("cargo:rustc-link-arg=-Tlinkall.x");
    }
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=ld/qemu-linkall.x");
    println!("cargo:rerun-if-changed=ld/alias-qemu.x");
}
