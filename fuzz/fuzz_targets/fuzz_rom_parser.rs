#![no_main]

use libfuzzer_sys::fuzz_target;
use nes_core::NesCore;

fuzz_target!(|data: &[u8]| {
    let mut core = NesCore::new();
    let _ = core.load_ines_rom(data);
});
