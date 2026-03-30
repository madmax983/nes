use proptest::prelude::*;
use std::str::FromStr;
use nes_core::CheatCode;

proptest! {
    #[test]
    #[ignore = "havoc target"]
    fn proptest_cheat_code_from_str(s in ".*") {
        let _ = CheatCode::from_str(&s);
    }

    #[test]
    #[ignore = "havoc target"]
    fn proptest_api_add_cheat_code(s in ".*") {
        let mut core = nes_core::NesCore::new();
        let _ = core.add_cheat_code(&s);
    }
}
