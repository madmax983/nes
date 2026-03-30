use nes_dsl::assemble;

#[test]
#[ignore = "havoc target"]
fn havoc_dsl_i64_min_panic() {
    let _ = assemble(".byte -9223372036854775808").unwrap();
}
