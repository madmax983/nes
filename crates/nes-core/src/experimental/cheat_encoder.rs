//! Experimental NES cheat code encoder.
//!
//! This module complements `CheatCode` by providing the ability to take an address,
//! a new value, and an optional compare byte, and mathematically shuffle them into
//! a 6- or 8-character NES cheat code string (Game Genie format).

#[cfg(feature = "nova")]
const CHEAT_CODE_ALPHABET: &[u8] = b"APZLGITYEOXUKSVN";

/// Experimental encoder to generate NES cheat codes.
///
/// This provides the inverse operation to `CheatCode::from_str`. It can turn raw
/// memory hacks (found using tools like `CheatFinder`) into shareable text codes.
#[cfg(feature = "nova")]
pub struct CheatEncoder;

#[cfg(feature = "nova")]
impl CheatEncoder {
    /// Encodes an address and a replacement value into a 6-character cheat code string.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_core::experimental::cheat_encoder::CheatEncoder;
    ///
    /// // Super Mario Bros: Infinite Lives
    /// let code = CheatEncoder::encode_6_char(0x9BE1, 0xAD);
    /// assert_eq!(code, "SXTPOU");
    /// ```
    #[must_use]
    pub fn encode_6_char(address: u16, value: u8) -> String {
        let mut digits = [0u8; 6];

        let a = address & 0x7FFF;

        digits[0] = (value & 0x7) | ((value >> 4) & 0x8);
        digits[1] = ((value >> 4) & 0x7) | (((a >> 4) & 0x8) as u8);
        digits[2] = ((a >> 4) & 0x7) as u8;
        digits[3] = (((a >> 12) & 0x7) | (a & 0x8)) as u8;
        digits[4] = ((a & 0x7) | ((a >> 8) & 0x8)) as u8;
        digits[5] = (((a >> 8) & 0x7) as u8) | (value & 0x8);

        let mut result = String::with_capacity(6);
        for &d in &digits {
            result.push(CHEAT_CODE_ALPHABET[(d & 0xF) as usize] as char);
        }
        result
    }

    /// Encodes an address, replacement value, and compare value into an 8-character cheat code string.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_core::experimental::cheat_encoder::CheatEncoder;
    ///
    /// let code = CheatEncoder::encode_8_char(0x94A7, 0x02, 0x03);
    /// assert_eq!(code, "ZEZPYGLA");
    /// ```
    #[must_use]
    pub fn encode_8_char(address: u16, value: u8, compare: u8) -> String {
        let mut digits = [0u8; 8];
        let a = address & 0x7FFF;

        digits[0] = (value & 0x7) | ((value >> 4) & 0x8);
        digits[1] = ((value >> 4) & 0x7) | (((a >> 4) & 0x8) as u8);
        digits[2] = ((a >> 4) & 0x7) as u8;
        digits[3] = (((a >> 12) & 0x7) | (a & 0x8)) as u8;
        digits[4] = ((a & 0x7) | ((a >> 8) & 0x8)) as u8;

        digits[5] = (((a >> 8) & 0x7) as u8) | (compare & 0x8);
        digits[6] = (compare & 0x7) | ((compare >> 4) & 0x8);
        digits[7] = ((compare >> 4) & 0x7) | (value & 0x8);

        let mut result = String::with_capacity(8);
        for &d in &digits {
            result.push(CHEAT_CODE_ALPHABET[(d & 0xF) as usize] as char);
        }
        result
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::CheatCode;
    use core::str::FromStr;

    #[test]
    fn test_encode_6_char() {
        let code = CheatEncoder::encode_6_char(0x9BE1, 0xAD);
        assert_eq!(code, "SXTPOU");

        // Verify it round trips through the standard decoder
        let decoded = CheatCode::from_str(&code).unwrap();
        assert_eq!(decoded.address(), 0x9BE1);
        assert_eq!(decoded.value(), 0xAD);
        assert_eq!(decoded.compare(), None);
    }

    #[test]
    fn test_encode_8_char() {
        let code = CheatEncoder::encode_8_char(0x94A7, 0x02, 0x03);
        assert_eq!(code, "ZEZPYGLA");

        // Verify it round trips through the standard decoder
        let decoded = CheatCode::from_str(&code).unwrap();
        assert_eq!(decoded.address(), 0x94A7);
        assert_eq!(decoded.value(), 0x02);
        assert_eq!(decoded.compare(), Some(0x03));
    }
}
