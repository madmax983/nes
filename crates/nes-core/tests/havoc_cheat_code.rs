use nes_core::CheatCode;
use proptest::prelude::*;
use std::str::FromStr;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_cheat_code_from_str(source in ".*") {
        let _ = CheatCode::from_str(&source);
    }
}
