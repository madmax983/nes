use proptest::prelude::*;
use std::str::FromStr;

proptest! {
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_cheat_code_parsing(s in ".*") {
        let _ = nes_core::CheatCode::from_str(&s);
    }
}
