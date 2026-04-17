use nes_dsl::assemble;

#[test]
fn test_handle_word_directive() {
    let source = r#"
        .org $C000
        .word $1234, $5678
        .reset $C000
    "#;
    let program = assemble(source).expect("assembly should succeed");
    assert_eq!(program.bytes.get(&0xC000), Some(&0x34));
    assert_eq!(program.bytes.get(&0xC001), Some(&0x12));
    assert_eq!(program.bytes.get(&0xC002), Some(&0x78));
    assert_eq!(program.bytes.get(&0xC003), Some(&0x56));
}

#[test]
fn test_handle_text_directive() {
    let source = r#"
        .org $C000
        .text "HELLO"
        .reset $C000
    "#;
    let program = assemble(source).expect("assembly should succeed");
    assert_eq!(program.bytes.get(&0xC000), Some(&b'H'));
    assert_eq!(program.bytes.get(&0xC001), Some(&b'E'));

    let source2 = r#"
        .org $C000
        .text
    "#;
    let res = assemble(source2);
    assert!(res.is_err());

    let source3 = r#"
        .org $C000
        .text NOTQUOTED
    "#;
    let res3 = assemble(source3);
    assert!(res3.is_err());
}

#[test]
fn test_handle_byte_directive() {
    let source = r#"
        .org $C000
        .byte $12, "AB"
        .reset $C000
    "#;
    let program = assemble(source).expect("assembly should succeed");
    assert_eq!(program.bytes.get(&0xC000), Some(&0x12));
    assert_eq!(program.bytes.get(&0xC001), Some(&b'A'));
    assert_eq!(program.bytes.get(&0xC002), Some(&b'B'));
}

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
