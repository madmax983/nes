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
        let normalized: String = raw
            .chars()
            .filter(|ch| !ch.is_ascii_whitespace() && *ch != '-')
            .map(|ch| ch.to_ascii_uppercase())
            .collect();

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
