use core::fmt;

const INES_HEADER_LEN: usize = 16;
const INES_TRAINER_LEN: usize = 512;
const PRG_BANK_BYTES: usize = 16 * 1024;
const CHR_BANK_BYTES: usize = 8 * 1024;

const INES_MAGIC: [u8; 4] = [0x4E, 0x45, 0x53, 0x1A];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NametableMirroring {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InesRom<'a> {
    pub mapper_id: u8,
    pub prg_rom: &'a [u8],
    pub chr_rom: &'a [u8],
    pub mirroring: NametableMirroring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RomError {
    InvalidMagic,
    UnsupportedFourScreenMirroring,
    UnsupportedNes2SizeEncoding,
    UnsupportedNes2ExtendedMapper(u16),
    MissingPrgRom,
    UnsupportedMapper(u8),
    UnsupportedPrgLayout(usize),
    Truncated { expected_min: usize, actual: usize },
}

impl fmt::Display for RomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => f.write_str("invalid iNES magic header"),
            Self::UnsupportedFourScreenMirroring => {
                f.write_str("four-screen nametable mirroring is not supported")
            }
            Self::UnsupportedNes2SizeEncoding => {
                f.write_str("NES 2.0 exponent/multiplier size encoding is not supported")
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
