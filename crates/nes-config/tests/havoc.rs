use nes_config::parse_config_path_arg;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_config_args(
        args in proptest::collection::vec(".*", 0..10),
    ) {
        let _ = parse_config_path_arg(&args);
    }
}
