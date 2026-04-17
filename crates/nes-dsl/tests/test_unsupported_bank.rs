use nes_dsl::assemble;

#[test]
fn test_unsupported_bank() {
    let source = r#"
        .bank 2
    "#;
    let res = assemble(source);
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(
        err.to_string()
            .contains("only `.bank 0` and `.bank 1` are supported")
    );
}
