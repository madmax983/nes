use proptest::prelude::*;
use nes_core::NesCore;
use nes_mcp::dispatch_tool;
use std::collections::BTreeMap;

proptest! {
    #[test]
    #[ignore = "havoc target"]
    fn proptest_dispatch_tool(name in ".*", params in proptest::collection::btree_map(".*", ".*", 0..10)) {
        let mut core = NesCore::new();
        let _ = dispatch_tool(&mut core, &name, &params);
    }
}
