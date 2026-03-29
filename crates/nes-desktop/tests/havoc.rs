use nes_desktop::args::parse_runtime_args;
use nes_desktop::session_cheats::SessionCheats;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_session_cheats(raw_code in ".*") {
        let mut cheats = SessionCheats::new();
        let _ = cheats.add(&raw_code);
    }

    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_session_cheats_multiple(
        codes in proptest::collection::vec(".*", 0..10),
    ) {
        let _ = SessionCheats::from_raw_codes(&codes);
    }

    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_desktop_args(
        args in proptest::collection::vec(".*", 0..10),
    ) {
        let _ = parse_runtime_args(&args);
    }
}
