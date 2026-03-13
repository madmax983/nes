#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(script) = std::str::from_utf8(data) {
        let mut core = nes_core::NesCore::new();
        let _ = nes_mcp::macro_engine::execute_macro_script(&mut core, script);
    }
});
