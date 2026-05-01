use nes_relay::config;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_config_parse_u64_arg(value in ".*", flag in ".*") {
        let _ = config::parse_u64_arg(&value, &flag);
    }

    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_config_parse_percent_arg(value in ".*", flag in ".*") {
        let _ = config::parse_percent_arg(&value, &flag);
    }
}
