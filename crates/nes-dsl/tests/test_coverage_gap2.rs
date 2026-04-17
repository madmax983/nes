use nes_dsl::assemble;

#[test]
fn test_resolve_addressing_mode() {
    let source = r#"
        .org $C000
        BRK         ; Implied
        ASL A       ; Accumulator
        LDA #$12    ; Immediate
        LDA ($12,X) ; IndirectX
        LDA ($12),Y ; IndirectY
        JMP ($1234) ; Indirect
        LDA $12,X   ; ZeroPageX
        LDA $1234,X ; AbsoluteX
        LDA $12,Y   ; ZeroPageY
        LDA $1234,Y ; AbsoluteY
        LDA $12     ; ZeroPage
        LDA $1234   ; Absolute
        .reset $C000
    "#;
    let _program = assemble(source).expect("assembly should succeed");
}
