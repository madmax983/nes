use nes_core::cheat_codes::{CheatCode};
use std::str::FromStr;

#[test]
fn test_cheat_code_decodes_compare_bits_bitwise() {
    // Tests for mutants in cheat codes
    let c1 = CheatCode::from_str("AAAAANNA").unwrap();
    assert_eq!(c1.compare(), Some(0x8F));

    // Address bit overlap
    let c2 = CheatCode::from_str("AAAAAN").unwrap();
    assert_eq!(c2.address(), 0x8700);
}
