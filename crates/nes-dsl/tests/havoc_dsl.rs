use nes_dsl::assemble;
use proptest::prelude::*;

proptest! {
    // Force a lot of cases and specifically test the edge bounds.
    #![proptest_config(ProptestConfig::with_cases(5000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_dsl_number(
        num in prop_oneof![
            Just(i64::MIN),
            Just(i64::MAX),
            any::<i64>(),
        ]
    ) {
        let dec = num.to_string();
        let source = format!(".byte {}", dec);

        let res = assemble(&source);
        if let Err(nes_dsl::DslError::Parse { message, .. }) = res {
            panic!("Parser failed on valid i64 decimal integer format! Num: {}, Dec: {}, Error: {}", num, dec, message);
        }
    }
}
