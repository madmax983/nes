use nes_core::NesCore;
use nes_mcp::{ToolParams, dispatch_tool};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    #[test]
    #[ignore = "havoc target"]
    fn test_fuzz_dispatch_parse_integer(ref source in ".*") {
        let mut core = NesCore::new();
        let mut params = ToolParams::new();
        params.insert("address".to_string(), source.clone());
        let _ = dispatch_tool(&mut core, "read_memory", &params);

        params.insert("rom_hex".to_string(), source.clone());
        let _ = dispatch_tool(&mut core, "load_rom", &params);

        params.insert("source".to_string(), source.clone());
        let _ = dispatch_tool(&mut core, "assemble_6502_dsl", &params);
    }
}
