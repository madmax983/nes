#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        if s.len() > 1024 { return; }
        if let Ok(_) = nes_dsl::assemble(s) {
            let options = nes_dsl::RomBuildOptions::default();
            let _ = nes_dsl::build_ines_nrom_rom(s, &options);
        }
    }
});
