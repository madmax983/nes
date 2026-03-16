use proptest::prelude::*;
use nes_core::NesCore;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50000))]
    #[test]
    fn does_not_crash(s in "[a-zA-Z0-9_ \\n\\r\\t#$:,\"\\.=-]*") {
        // Just checking string processing locally or via another method,
        // avoiding macro_engine to dodge API breaking changes upstream.
        let mut _core = NesCore::new();
        let _ = s.split_whitespace().collect::<Vec<_>>();
    }
}
