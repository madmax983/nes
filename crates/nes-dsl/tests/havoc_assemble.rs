use proptest::prelude::*;
use nes_dsl::assemble;

proptest! {
    #[test]
    #[ignore = "havoc target"]
    fn proptest_assemble(s in ".*") {
        let _ = assemble(&s);
    }
}
