//! NES letter-encoded cheat code decoding and matching.
//!
//! The bit shuffles here follow the documented 6- and 8-character NES cheat
//! code tables: each character contributes a 4-bit nybble from the
//! `APZLGITYEOXUKSVN` alphabet, then address/data/compare bits are
//! reassembled from those nybbles.

use core::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

const CHEAT_CODE_ALPHABET: &str = "APZLGITYEOXUKSVN";

/// Decoded NES cheat patch.
///
/// This struct holds the parsed representation of a standard NES "Game Genie" cheat code.
/// It decodes 6-character and 8-character codes into the underlying address, value,
/// and optional compare byte.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use nes_core::CheatCode;
///
/// // Parse a 6-character code (Super Mario Bros: Infinite Lives)
/// let code = CheatCode::from_str("SXTPOU").unwrap();
/// assert_eq!(code.address(), 0x9BE1);
/// assert_eq!(code.value(), 0xAD);
/// assert_eq!(code.compare(), None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheatCode {
    raw: String,
    address: u16,
    value: u8,
    compare: Option<u8>,
}

impl CheatCode {
    /// Returns the normalized uppercase code string.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Returns the CPU PRG-space address targeted by this code.
    #[must_use]
    pub const fn address(&self) -> u16 {
        self.address
    }

    /// Returns the replacement byte injected when the code matches.
    #[must_use]
    pub const fn value(&self) -> u8 {
        self.value
    }

    /// Returns the optional compare byte for 8-character codes.
    #[must_use]
    pub const fn compare(&self) -> Option<u8> {
        self.compare
    }

    pub(crate) fn applies_to(&self, addr: u16, original: u8) -> bool {
        self.address == addr && self.compare.is_none_or(|compare| compare == original)
    }
}

impl FromStr for CheatCode {
    type Err = CheatCodeError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        // **Performance optimization:** Instead of using `.filter(...).map(...).collect::<String>()`
        // which cannot predict the final length and may re-allocate the heap buffer multiple times,
        // we pre-allocate exactly 8 bytes (the maximum valid length of a NES cheat code)
        // and push characters manually. This guarantees a single, zero-overhead allocation.
        let mut normalized = String::with_capacity(8);
        for ch in raw.chars() {
            if ch.is_ascii_whitespace() || ch == '-' {
                continue;
            }
            normalized.push(ch.to_ascii_uppercase());
        }

        if normalized.len() != 6 && normalized.len() != 8 {
            return Err(CheatCodeError::InvalidLength(normalized.len()));
        }

        let mut digits = [0u8; 8];
        for (index, ch) in normalized.chars().enumerate() {
            let Some(value) = alphabet_digit(ch) else {
                return Err(CheatCodeError::InvalidCharacter { ch, index });
            };
            digits[index] = value;
        }

        let address = 0x8000
            | (u16::from(digits[3] & 0x7) << 12)
            | (u16::from(digits[5] & 0x7) << 8)
            | (u16::from(digits[4] & 0x8) << 8)
            | (u16::from(digits[2] & 0x7) << 4)
            | (u16::from(digits[1] & 0x8) << 4)
            | u16::from(digits[4] & 0x7)
            | u16::from(digits[3] & 0x8);
        let value = ((digits[1] & 0x7) << 4)
            | ((digits[0] & 0x8) << 4)
            | (digits[0] & 0x7)
            | (digits[5] & 0x8);
        let compare = if normalized.len() == 8 {
            Some(
                ((digits[7] & 0x7) << 4)
                    | ((digits[6] & 0x8) << 4)
                    | (digits[6] & 0x7)
                    | (digits[5] & 0x8),
            )
        } else {
            None
        };

        Ok(Self {
            raw: normalized,
            address,
            value,
            compare,
        })
    }
}

/// Errors returned when parsing a cheat code string.
///
/// This error type is returned by [`CheatCode::from_str`] when a provided
/// cheat code string is malformed. It provides specific information about
/// whether the length was incorrect or if invalid characters were used,
/// which is useful for displaying helpful feedback to the user.
///
/// # Examples
///
/// ```
/// use std::str::FromStr;
/// use nes_core::{CheatCode, CheatCodeError};
///
/// // Attempt to parse a code with invalid characters
/// let err = CheatCode::from_str("123456").unwrap_err();
///
/// match err {
///     CheatCodeError::InvalidCharacter { ch, index } => {
///         assert_eq!(ch, '1');
///         assert_eq!(index, 0);
///     }
///     _ => panic!("Expected InvalidCharacter"),
/// }
///
/// // Attempt to parse a code with invalid length
/// let err = CheatCode::from_str("SXTPO").unwrap_err();
/// assert_eq!(err, CheatCodeError::InvalidLength(5));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheatCodeError {
    /// Normalized code length was not 6 or 8 characters.
    InvalidLength(usize),
    /// Code used a character outside the supported alphabet.
    InvalidCharacter {
        /// Unexpected character.
        ch: char,
        /// Zero-based character index in the normalized code.
        index: usize,
    },
}

impl fmt::Display for CheatCodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLength(len) => {
                write!(f, "cheat code length must be 6 or 8 characters, got {len}")
            }
            Self::InvalidCharacter { ch, index } => write!(
                f,
                "invalid cheat code character '{ch}' at position {index}; expected letters from {CHEAT_CODE_ALPHABET}"
            ),
        }
    }
}

impl std::error::Error for CheatCodeError {}

fn alphabet_digit(ch: char) -> Option<u8> {
    CHEAT_CODE_ALPHABET
        .chars()
        .position(|candidate| candidate == ch)
        .and_then(|index| u8::try_from(index).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheat_code_parser_ignores_whitespace_and_hyphens() {
        let code1 = CheatCode::from_str("S X T P O U").unwrap();
        assert_eq!(code1.raw(), "SXTPOU");
        assert_eq!(code1.address(), 0x9BE1);

        let code2 = CheatCode::from_str("S-X-T-P-O-U").unwrap();
        assert_eq!(code2.raw(), "SXTPOU");
        assert_eq!(code2.address(), 0x9BE1);

        let code3 = CheatCode::from_str("S X-T P-O U").unwrap();
        assert_eq!(code3.raw(), "SXTPOU");
        assert_eq!(code3.address(), 0x9BE1);

        let code_only_hyphens = CheatCode::from_str("SXT-POU").unwrap();
        assert_eq!(code_only_hyphens.raw(), "SXTPOU");

        let code_only_spaces = CheatCode::from_str("SXT POU").unwrap();
        assert_eq!(code_only_spaces.raw(), "SXTPOU");
    }

    #[test]
    fn cheat_code_parser_fails_with_invalid_characters() {
        let err = CheatCode::from_str("123456").unwrap_err();
        assert_eq!(err, CheatCodeError::InvalidCharacter { ch: '1', index: 0 });

        let err2 = CheatCode::from_str("SXTPO1").unwrap_err();
        assert_eq!(err2, CheatCodeError::InvalidCharacter { ch: '1', index: 5 });
    }

    #[test]
    fn cheat_code_applies_to_logic() {
        let code_6 = CheatCode::from_str("SXTPOU").unwrap();
        // Applies to exact address
        assert!(code_6.applies_to(0x9BE1, 0xFF));
        // Doesn't apply to wrong address
        assert!(!code_6.applies_to(0x9BE2, 0xFF));

        let code_8 = CheatCode::from_str("ZEXPYGLA").unwrap();
        // Applies to exact address and matching original value
        assert!(code_8.applies_to(0x94A7, 0x03));
        // Doesn't apply if original value doesn't match compare byte
        assert!(!code_8.applies_to(0x94A7, 0x04));
        // Doesn't apply if address is wrong
        assert!(!code_8.applies_to(0x94A8, 0x03));
    }

    #[test]
    fn cheat_code_error_formatting() {
        let err_len = CheatCodeError::InvalidLength(5);
        assert_eq!(
            err_len.to_string(),
            "cheat code length must be 6 or 8 characters, got 5"
        );

        let err_char = CheatCodeError::InvalidCharacter { ch: '1', index: 0 };
        assert_eq!(
            err_char.to_string(),
            format!(
                "invalid cheat code character '1' at position 0; expected letters from {}",
                CHEAT_CODE_ALPHABET
            )
        );
    }

    #[test]
    fn invalid_cheat_codes_return_errors() {
        let err_5 = CheatCode::from_str("SXTPO").unwrap_err();
        assert_eq!(err_5, CheatCodeError::InvalidLength(5));

        let err_7 = CheatCode::from_str("ZEXPYGL").unwrap_err();
        assert_eq!(err_7, CheatCodeError::InvalidLength(7));

        let err_9 = CheatCode::from_str("ZEXPYGLAZ").unwrap_err();
        assert_eq!(err_9, CheatCodeError::InvalidLength(9));

        let err_empty = CheatCode::from_str("").unwrap_err();
        assert_eq!(err_empty, CheatCodeError::InvalidLength(0));

        let err_spaces = CheatCode::from_str("   ").unwrap_err();
        assert_eq!(err_spaces, CheatCodeError::InvalidLength(0));

        let err_hyphens = CheatCode::from_str("---").unwrap_err();
        assert_eq!(err_hyphens, CheatCodeError::InvalidLength(0));

        let err_mixed_empty = CheatCode::from_str("- - -").unwrap_err();
        assert_eq!(err_mixed_empty, CheatCodeError::InvalidLength(0));
    }

    #[test]
    fn cheat_code_accessors_return_correct_values() {
        let code = CheatCode::from_str("SXTPOU").unwrap();
        assert_eq!(code.raw(), "SXTPOU");
        assert_eq!(code.address(), 0x9BE1);
        assert_eq!(code.value(), 0xAD);
        assert_eq!(code.compare(), None);

        let code_8 = CheatCode::from_str("ZEXPYGLA").unwrap();
        assert_eq!(code_8.raw(), "ZEXPYGLA");
        assert_eq!(code_8.address(), 0x94A7);
        assert_eq!(code_8.value(), 0x02);
        assert_eq!(code_8.compare(), Some(0x03));
    }
}
