use nes_dsl::assemble;

#[test]
fn test_parse_uppercase_hex() {
    let result = assemble(".org $8000\nreset:\n.byte 0X12\n.org $fffc\n.word reset");
    assert!(result.is_ok());
}
