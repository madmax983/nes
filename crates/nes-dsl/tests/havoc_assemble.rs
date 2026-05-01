use nes_dsl::assemble;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100000))]
    #[test]
    #[ignore = "havoc target"]
    fn havoc_fuzz_assemble(source in ".*") {
        let _ = assemble(&source);
    }
}
