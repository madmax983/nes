use nes_dsl::assemble;

#[test]
fn test_instruction_error() {
    let source = r#"
        .org $C000
        WUT
    "#;
    let res = assemble(source);
    assert!(res.is_err());
    let err = res.expect_err("expected error");
    assert!(err.to_string().contains("unknown mnemonic 'WUT'"));
}

#[test]
fn test_instruction_mode_error() {
    let source = r#"
        .org $C000
        LDA
    "#;
    let res = assemble(source);
    assert!(res.is_err());
    let err = res.expect_err("expected error");
    assert!(
        err.to_string()
            .contains("addressing mode implied not supported for LDA")
    );
}

#[test]
fn test_instruction_branch_no_target() {
    let source = r#"
        .org $C000
        BNE
    "#;
    let res = assemble(source);
    assert!(res.is_err());
    let err = res.expect_err("expected error");
    assert!(err.to_string().contains("BNE expects a target operand"));
}
