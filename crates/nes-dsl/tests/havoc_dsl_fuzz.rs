use nes_dsl::assemble;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50000))]
    #[test]
    fn havoc_fuzz_assemble(source in ".*") {
        let _ = assemble(&source);
    }
}
