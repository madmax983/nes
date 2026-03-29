use nes_core::NesCore;
use nes_mcp::macro_engine::execute_macro_script;
use nes_mcp::{ToolParams, dispatch_tool};
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_mcp_macro(script in ".*") {
        let mut core = NesCore::new();
        let _ = execute_macro_script(&mut core, &script, None);
    }

    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_mcp_params(
        tool_name in ".*",
        keys in proptest::collection::vec(".*", 0..5),
        values in proptest::collection::vec(".*", 0..5),
    ) {
        let mut core = NesCore::new();
        let mut params = ToolParams::new();
        for (k, v) in keys.into_iter().zip(values.into_iter()) {
            params.insert(k, v);
        }
        let _ = dispatch_tool(&mut core, &tool_name, &params);
    }
}
