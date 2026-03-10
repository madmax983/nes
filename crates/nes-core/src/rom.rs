//! ROM parsing and iNES/NES2 header interpretation.
//!
//! This module extracts mapper metadata and PRG/CHR slices from raw `.nes`
//! bytes. Parsing is intentionally strict: unsupported layouts or header
//! features return explicit [`RomError`] values.

use core::fmt;

use serde::{Deserialize, Serialize};

const INES_HEADER_LEN: usize = 16;
const INES_TRAINER_LEN: usize = 512;
const PRG_BANK_BYTES: usize = 16 * 1024;
const CHR_BANK_BYTES: usize = 8 * 1024;

const INES_MAGIC: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Nametable mirroring mode declared by the cartridge header.
pub enum NametableMirroring {
    /// Horizontal arrangement (`[A A][B B]`).
    Horizontal,
    /// Vertical arrangement (`[A B][A B]`).
    Vertical,
    /// One-screen mirroring to first nametable page.
    OneScreenLower,
    /// One-screen mirroring to second nametable page.
    OneScreenUpper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Parsed iNES ROM view backed by the original ROM byte slice.
pub struct InesRom<'a> {
    /// iNES mapper number.
    pub mapper_id: u8,
    /// PRG ROM data slice.
    pub prg_rom: &'a [u8],
    /// CHR ROM data slice (empty when CHR RAM is used).
    pub chr_rom: &'a [u8],
    /// Cartridge nametable mirroring mode.
    pub mirroring: NametableMirroring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Structured ROM loading failures.
pub enum RomError {
    /// File does not start with `NES<EOF>`.
    InvalidMagic,
    /// Console type is unsupported (VS/PlayChoice/etc).
    UnsupportedConsoleType(u8),
    /// Four-screen mirroring is not implemented yet.
    UnsupportedFourScreenMirroring,
    /// NES 2.0 exponent/multiplier sizing is not implemented.
    UnsupportedNes2SizeEncoding,
    /// NES 2.0 submapper is not currently supported.
    UnsupportedNes2Submapper(u8),
    /// NES 2.0 extended mapper IDs are currently unsupported.
    UnsupportedNes2ExtendedMapper(u16),
    /// PRG bank count is zero.
    MissingPrgRom,
    /// Mapper is outside the supported set.
    UnsupportedMapper(u8),
    /// PRG size does not match mapper constraints.
    UnsupportedPrgLayout(usize),
    /// File ended before required PRG/CHR bytes were available.
    Truncated { expected_min: usize, actual: usize },
}

impl fmt::Display for RomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => f.write_str("invalid iNES magic header"),
            Self::UnsupportedConsoleType(console_type) => {
                write!(f, "unsupported console type {console_type}")
            }
            Self::UnsupportedFourScreenMirroring => {
                f.write_str("four-screen nametable mirroring is not supported")
            }
            Self::UnsupportedNes2SizeEncoding => {
                f.write_str("NES 2.0 exponent/multiplier size encoding is not supported")
            }
            Self::UnsupportedNes2Submapper(submapper) => {
                write!(f, "NES 2.0 submapper {submapper} is not supported")
            }
            Self::UnsupportedNes2ExtendedMapper(mapper) => {
                write!(f, "NES 2.0 extended mapper id {mapper} is not supported")
            }
            Self::MissingPrgRom => f.write_str("ROM has no PRG banks"),
            Self::UnsupportedMapper(mapper) => write!(f, "unsupported mapper id {mapper}"),
            Self::UnsupportedPrgLayout(bytes) => write!(f, "unsupported PRG layout size {bytes}"),
            Self::Truncated {
                expected_min,
                actual,
            } => write!(
                f,
                "truncated ROM: expected at least {expected_min} bytes, got {actual}"
            ),
        }
    }
}

impl std::error::Error for RomError {}

/// Parses an iNES/NES2 ROM image and returns a borrowed [`InesRom`] view.
///
/// # Errors
///
/// Returns [`RomError`] when header bytes are invalid, sizes are unsupported,
/// or the input is truncated.
pub fn parse_ines(bytes: &[u8]) -> Result<InesRom<'_>, RomError> {
    if bytes.len() < INES_HEADER_LEN {
        return Err(RomError::Truncated {
            expected_min: INES_HEADER_LEN,
            actual: bytes.len(),
        });
    }

    if bytes[0..4] != INES_MAGIC {
        return Err(RomError::InvalidMagic);
    }

    let flags6 = bytes[6];
    let flags7 = bytes[7];
    let is_nes2 = (flags7 & 0b0000_1100) == 0b0000_1000;
    let console_type = flags7 & 0b0000_0011;
    if console_type != 0 {
        return Err(RomError::UnsupportedConsoleType(console_type));
    }
    if flags6 & 0b0000_1000 != 0 {
        return Err(RomError::UnsupportedFourScreenMirroring);
    }
    let mirroring = if flags6 & 0b0000_0001 != 0 {
        NametableMirroring::Vertical
    } else {
        NametableMirroring::Horizontal
    };

    let (mapper_id, prg_banks, chr_banks) = if is_nes2 {
        let mapper_low = (flags6 >> 4) | (flags7 & 0xF0);
        let mapper_high = bytes[8] & 0x0F;
        let mapper_extended = ((mapper_high as u16) << 8) | mapper_low as u16;
        let submapper = bytes[8] >> 4;
        if submapper != 0 {
            return Err(RomError::UnsupportedNes2Submapper(submapper));
        }
        if mapper_high != 0 {
            return Err(RomError::UnsupportedNes2ExtendedMapper(mapper_extended));
        }

        let prg_msb = bytes[9] & 0x0F;
        let chr_msb = (bytes[9] >> 4) & 0x0F;
        if prg_msb == 0x0F || chr_msb == 0x0F {
            return Err(RomError::UnsupportedNes2SizeEncoding);
        }

        (
            mapper_low,
            bytes[4] as usize | ((prg_msb as usize) << 8),
            bytes[5] as usize | ((chr_msb as usize) << 8),
        )
    } else {
        (
            flags6 >> 4 | (flags7 & 0xF0),
            bytes[4] as usize,
            bytes[5] as usize,
        )
    };

    if prg_banks == 0 {
        return Err(RomError::MissingPrgRom);
    }

    let trainer_bytes = if flags6 & 0b0000_0100 != 0 {
        INES_TRAINER_LEN
    } else {
        0
    };

    let prg_rom_len = prg_banks * PRG_BANK_BYTES;
    let chr_rom_len = chr_banks * CHR_BANK_BYTES;
    let prg_start = INES_HEADER_LEN + trainer_bytes;
    let expected_min = prg_start + prg_rom_len + chr_rom_len;

    if bytes.len() < expected_min {
        return Err(RomError::Truncated {
            expected_min,
            actual: bytes.len(),
        });
    }

    let prg_end = prg_start + prg_rom_len;
    let chr_end = prg_end + chr_rom_len;
    Ok(InesRom {
        mapper_id,
        prg_rom: &bytes[prg_start..prg_end],
        chr_rom: &bytes[prg_end..chr_end],
        mirroring,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rom_error_display() {
        assert_eq!(
            RomError::InvalidMagic.to_string(),
            "invalid iNES magic header"
        );
        assert_eq!(
            RomError::UnsupportedConsoleType(1).to_string(),
            "unsupported console type 1"
        );
        assert_eq!(
            RomError::UnsupportedFourScreenMirroring.to_string(),
            "four-screen nametable mirroring is not supported"
        );
        assert_eq!(
            RomError::UnsupportedNes2SizeEncoding.to_string(),
            "NES 2.0 exponent/multiplier size encoding is not supported"
        );
        assert_eq!(
            RomError::UnsupportedNes2Submapper(3).to_string(),
            "NES 2.0 submapper 3 is not supported"
        );
        assert_eq!(
            RomError::UnsupportedNes2ExtendedMapper(42).to_string(),
            "NES 2.0 extended mapper id 42 is not supported"
        );
        assert_eq!(RomError::MissingPrgRom.to_string(), "ROM has no PRG banks");
        assert_eq!(
            RomError::UnsupportedMapper(99).to_string(),
            "unsupported mapper id 99"
        );
        assert_eq!(
            RomError::UnsupportedPrgLayout(8192).to_string(),
            "unsupported PRG layout size 8192"
        );
        assert_eq!(
            RomError::Truncated {
                expected_min: 100,
                actual: 50,
            }
            .to_string(),
            "truncated ROM: expected at least 100 bytes, got 50"
        );
    }

    #[test]
    fn test_parse_ines_exactly_16_bytes_missing_prg() {
        let mut bytes = [0; 16];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[4] = 0; // 0 PRG banks
        let err = parse_ines(&bytes).unwrap_err();
        // If `< 16` is mutated to `<= 16`, it will incorrectly return Truncated { 16, 16 }
        assert_eq!(err, RomError::MissingPrgRom);
    }

    #[test]
    fn test_parse_ines_truncated_header() {
        let err = parse_ines(&[0; 15]).unwrap_err();
        assert_eq!(
            err,
            RomError::Truncated {
                expected_min: 16,
                actual: 15
            }
        );
    }

    #[test]
    fn test_parse_ines_truncated_body() {
        let mut bytes = vec![0; 32];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[4] = 1; // 1 PRG bank (16KB)
        bytes[5] = 1; // 1 CHR bank (8KB)

        let err = parse_ines(&bytes).unwrap_err();
        assert_eq!(
            err,
            RomError::Truncated {
                expected_min: 16 + 16384 + 8192,
                actual: 32,
            }
        );
    }

    #[test]
    fn test_parse_ines_invalid_magic() {
        let mut bytes = [0; 32];
        bytes[0..4].copy_from_slice(b"BAD!");
        let err = parse_ines(&bytes).unwrap_err();
        assert_eq!(err, RomError::InvalidMagic);
    }

    #[test]
    fn test_parse_ines_four_screen_mirroring() {
        let mut bytes = [0; 32];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[6] = 0b0000_1000;
        let err = parse_ines(&bytes).unwrap_err();
        assert_eq!(err, RomError::UnsupportedFourScreenMirroring);
    }

    #[test]
    fn test_parse_ines_mirroring() {
        let mut bytes = [0; 32];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[4] = 1; // 1 PRG bank

        // Vertical mirroring
        bytes[6] = 0b0000_0001;
        let mut rom_bytes = vec![0; 16 + 16 * 1024];
        rom_bytes[0..16].copy_from_slice(&bytes[0..16]);
        let rom = parse_ines(&rom_bytes).unwrap();
        assert_eq!(rom.mirroring, NametableMirroring::Vertical);

        // Horizontal mirroring
        bytes[6] = 0b0000_0000;
        rom_bytes[0..16].copy_from_slice(&bytes[0..16]);
        let rom = parse_ines(&rom_bytes).unwrap();
        assert_eq!(rom.mirroring, NametableMirroring::Horizontal);
    }

    #[test]
    fn test_parse_ines_missing_prg() {
        let mut bytes = [0; 32];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[4] = 0; // 0 PRG banks
        let err = parse_ines(&bytes).unwrap_err();
        assert_eq!(err, RomError::MissingPrgRom);
    }

    #[test]
    fn test_parse_ines_nes2_extended_mapper() {
        let mut bytes = [0; 32];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[7] = 0b0000_1000; // NES 2.0 marker
        bytes[8] = 0x01; // Extended mapper > 0
        let err = parse_ines(&bytes).unwrap_err();
        assert_eq!(err, RomError::UnsupportedNes2ExtendedMapper(0x0100));
    }

    #[test]
    fn test_parse_ines_nes2_size_encoding() {
        let mut bytes = [0; 32];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[7] = 0b0000_1000; // NES 2.0 marker
        bytes[9] = 0x0F; // PRG MSB == 0x0F
        let err = parse_ines(&bytes).unwrap_err();
        assert_eq!(err, RomError::UnsupportedNes2SizeEncoding);

        bytes[9] = 0xF0; // CHR MSB == 0x0F
        let err2 = parse_ines(&bytes).unwrap_err();
        assert_eq!(err2, RomError::UnsupportedNes2SizeEncoding);
    }

    #[test]
    fn test_parse_ines_nes2_submapper_is_rejected() {
        let mut bytes = vec![0; 16 + 16 * 1024];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[4] = 1; // 1 PRG bank
        bytes[7] = 0b0000_1000; // NES 2.0 marker
        bytes[8] = 0x10; // Submapper 1
        let err = parse_ines(&bytes).unwrap_err();
        assert_eq!(err, RomError::UnsupportedNes2Submapper(1));
    }

    #[test]
    fn test_parse_ines_ines_console_type_is_rejected() {
        let mut bytes = vec![0; 16 + 16 * 1024];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[4] = 1; // 1 PRG bank
        bytes[7] = 0b0000_0001; // VS Unisystem
        let err = parse_ines(&bytes).unwrap_err();
        assert_eq!(err, RomError::UnsupportedConsoleType(1));
    }

    #[test]
    fn test_parse_ines_nes2_console_type_is_rejected() {
        let mut bytes = vec![0; 16 + 16 * 1024];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[4] = 1; // 1 PRG bank
        bytes[7] = 0b0000_1001; // NES 2.0 marker + non-standard console type
        let err = parse_ines(&bytes).unwrap_err();
        assert_eq!(err, RomError::UnsupportedConsoleType(1));
    }

    #[test]
    fn test_parse_ines_nes2_mapper_id() {
        let mut bytes = vec![0; 16 + 16 * 1024];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[4] = 1; // 1 PRG bank
        bytes[7] = 0b0000_1000; // NES 2.0 marker

        // mapper_low = (flags6 >> 4) | (flags7 & 0xF0)
        // Set flags6 >> 4 to 0x05 (lower 4 bits of mapper id)
        // Set flags7 & 0xF0 to 0x20 (upper 4 bits of mapper id)
        // Resulting mapper_low should be 0x25 (37)
        bytes[6] = 0x50; // 0x50 >> 4 = 0x05
        bytes[7] |= 0x20;

        let rom = parse_ines(&bytes).unwrap();
        assert_eq!(rom.mapper_id, 0x25);
    }

    #[test]
    fn test_parse_ines_nes2_prg_chr_msb() {
        // We will construct a NES 2.0 ROM with:
        // prg_banks = 0x0102 (258 banks)
        // chr_banks = 0x0201 (513 banks)
        // prg_rom length = 258 * 16384 = 4,227,072 bytes
        // chr_rom length = 513 * 8192 = 4,202,496 bytes
        let prg_banks = 0x0102;
        let chr_banks = 0x0201;
        let prg_len = prg_banks * PRG_BANK_BYTES;
        let chr_len = chr_banks * CHR_BANK_BYTES;

        let mut bytes = vec![0; 16 + prg_len + chr_len];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[4] = (prg_banks & 0xFF) as u8; // 0x02
        bytes[5] = (chr_banks & 0xFF) as u8; // 0x01
        bytes[7] = 0b0000_1000; // NES 2.0 marker

        // bytes[9] = (chr_msb << 4) | prg_msb
        // prg_msb = 0x01, chr_msb = 0x02
        bytes[9] = (0x02 << 4) | 0x01;

        let rom = parse_ines(&bytes).unwrap();
        assert_eq!(rom.prg_rom.len(), prg_len);
        assert_eq!(rom.chr_rom.len(), chr_len);
    }

    #[test]
    fn test_parse_ines_trainer() {
        let mut bytes = vec![0; 16 + 512 + 16384];
        bytes[0..4].copy_from_slice(&INES_MAGIC);
        bytes[4] = 1; // 1 PRG bank
        bytes[6] = 0b0000_0100; // Has trainer
        let rom = parse_ines(&bytes).unwrap();
        assert_eq!(rom.prg_rom.len(), 16384);
        assert_eq!(
            rom.prg_rom.as_ptr() as usize - bytes.as_ptr() as usize,
            16 + 512
        );
    }
}
