use nes_desktop::args;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_args(args in proptest::collection::vec(".*", 0..10)) {
        let _ = args::parse_runtime_args(&args);
    }
}
