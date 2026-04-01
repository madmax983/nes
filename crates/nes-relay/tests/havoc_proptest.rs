use nes_relay::config::parse_args;
use proptest::prelude::*;

proptest! {
    #[test]
    #[ignore = "Havoc Input Attack"]
    #[should_panic]
    fn havoc_test_parse_args_overflow(
        latency in any::<u64>(),
        jitter in any::<u64>(),
        loss in any::<u64>(),
        reorder in any::<u64>()
    ) {
        // Generate inputs that might exceed percent bounds or number bounds
        let args = vec![
            "--bind".to_owned(),
            "127.0.0.1:0".to_owned(),
            "--latency-ms".to_owned(),
            format!("{}999", latency), // string suffix to cause potential panic if unchecked
            "--jitter-ms".to_owned(),
            format!("{}999", jitter),
            "--loss-pct".to_owned(),
            loss.to_string(), // valid parsing, but values > 100 should panic according to havoc?
            "--reorder-pct".to_owned(),
            reorder.to_string(),
        ];

        let _ = parse_args(args).unwrap(); // This will panic because parse_args returns an Err on failure. The proptest succeeds when it panics.
    }
}
