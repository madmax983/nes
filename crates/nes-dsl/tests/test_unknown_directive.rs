use nes_dsl::assemble;

#[test]
fn test_unknown_directive() {
    let source = r#"
        .unknown_directive
    "#;
    let res = assemble(source);
    assert!(res.is_err());
    let err = res.expect_err("expected error");
    assert!(
        err.to_string()
            .contains("unknown directive '.unknown_directive'")
    );
}
