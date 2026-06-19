use nes_dsl::assemble;
use proptest::prelude::*;

proptest! {
    #[test]
    #[should_panic(expected = "multiply with overflow")]
    fn havoc_proptest_math_panic(val in Just("-9223372036854775808")) {
        let source = format!(".org -{}", val);
        let _ = assemble(&source);
    }
}
