use nes_core::NesCore;
use nes_mcp::{ToolParams, dispatch_tool};
use proptest::prelude::*;

proptest! {
    #[test]
    fn havoc_test_load_rom_hex_crash(hex in "[A-Fa-f0-9 ]{10,1000}") {
        let mut core = NesCore::new();
        let mut params = ToolParams::new();
        params.insert("rom_hex".to_owned(), hex);
        let _ = dispatch_tool(&mut core, "load_rom", &params);
    }
}
