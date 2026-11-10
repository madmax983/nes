//! Build-time configuration for nes-core.
//!
//! `NES_QEMU_APU_STUB`: when set (to anything but `0`/empty) by the
//! ESP32-S3 QEMU firmware build, the APU mixer is stubbed to silence via
//! the `qemu_apu_stub` cfg. Espressif's QEMU fork faults on the mixer's
//! float instructions — the guest dies with an IllegalInstruction panic
//! before the first frame completes (reproduced 2026-09 and 2026-11) —
//! so the QEMU boot test cannot execute them.
//!
//! This is an environment variable rather than a cargo feature on
//! purpose: a feature would also be enabled by host
//! `cargo --all-features` builds, where the real mixer must stay
//! exercised (the APU hardware-fidelity tests assert that channel
//! writes change the mixed output). Only the QEMU firmware build
//! invocations (run-qemu-s3.sh, esp32s3-qemu.yml) set it.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(qemu_apu_stub)");
    println!("cargo::rerun-if-env-changed=NES_QEMU_APU_STUB");
    let stub = std::env::var("NES_QEMU_APU_STUB")
        .map(|v| v != "0" && !v.is_empty())
        .unwrap_or(false);
    if stub {
        println!("cargo::rustc-cfg=qemu_apu_stub");
    }
}
