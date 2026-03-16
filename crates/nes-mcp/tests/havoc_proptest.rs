use proptest::prelude::*;
use nes_core::NesCore;
use nes_mcp::macro_engine::execute_macro_script;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50000))]
    #[test]
    fn does_not_crash(s in "[a-zA-Z0-9_ \\n\\r\\t#$:,\"\\.=-]*") {
        let mut core = NesCore::new();
        let _ = execute_macro_script(&mut core, &s);
    }

    #[test]
    fn test_arbitrary_strings(s in ".*") {
        let mut core = NesCore::new();
        let _ = execute_macro_script(&mut core, &s);
    }
}
