use nes_mcp::ToolParams;
use nes_mcp::dispatch_tool;
use nes_core::NesCore;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn havoc_fuzz_dispatch_parse_integer(raw in ".*") {
        let mut core = NesCore::new();
        let mut params = ToolParams::new();
        params.insert("frame".to_string(), raw.clone());
        let _ = dispatch_tool(&mut core, "get_frame", &params);
    }
}
