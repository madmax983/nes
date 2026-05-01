use nes_desktop::session_cheats::SessionCheats;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_session_cheats_add(code in ".*") {
        let mut cheats = SessionCheats::new();
        let _ = cheats.add(&code);
    }
}
