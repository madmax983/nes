use super::Mapper;
use serde::{Deserialize, Serialize};

const PRG_16K_BYTES: usize = 16 * 1024;
const PRG_32K_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Mapper 0 (NROM): fixed PRG mapping with optional 16K mirroring.
pub struct Nrom {
    prg_rom: Vec<u8>,
}

impl Nrom {
    /// Builds a synthetic 32K NROM image with byte pattern data.
    ///
    /// Primarily useful in tests.
    #[must_use]
    pub fn new_32k() -> Self {
        let mut prg_rom = vec![0_u8; 32 * 1024];
        for (idx, byte) in prg_rom.iter_mut().enumerate() {
            *byte = (idx & 0xFF) as u8;
        }
        Self { prg_rom }
    }

    /// Builds NROM from raw PRG ROM bytes.
    ///
    /// Input is normalized to hardware-sized PRG windows:
    /// - `0..=16KB` becomes a 16KB NROM-128 image (zero-padded).
    /// - `16KB+1..=32KB` becomes a 32KB NROM-256 image (zero-padded).
    /// - `>32KB` is truncated to 32KB.
    #[must_use]
    pub fn from_prg_rom(mut prg_rom: Vec<u8>) -> Self {
        if prg_rom.len() <= PRG_16K_BYTES {
            prg_rom.resize(PRG_16K_BYTES, 0);
        } else if prg_rom.len() <= PRG_32K_BYTES {
            prg_rom.resize(PRG_32K_BYTES, 0);
        } else {
            prg_rom.truncate(PRG_32K_BYTES);
        }
        Self { prg_rom }
    }

    /// Reads PRG using NROM fixed mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Writes to PRG space (ignored by NROM hardware).
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }

    fn offset_for(&self, addr: u16) -> usize {
        let base = addr.saturating_sub(0x8000) as usize;
        base % self.prg_rom.len()
    }
}

impl Mapper for Nrom {
    fn read_prg(&self, addr: u16) -> u8 {
        self.prg_rom[self.offset_for(addr)]
    }

    fn write_prg(&mut self, _addr: u16, _value: u8) {
        // NROM has fixed PRG bank mapping and ignores bank-select writes.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nrom_from_prg_rom_exact_16k() {
        let prg_rom = vec![0; PRG_16K_BYTES];
        let nrom = Nrom::from_prg_rom(prg_rom);
        assert_eq!(nrom.prg_rom.len(), PRG_16K_BYTES);
    }

    #[test]
    fn test_nrom_from_prg_rom_exact_32k() {
        let prg_rom = vec![0; PRG_32K_BYTES];
        let nrom = Nrom::from_prg_rom(prg_rom);
        assert_eq!(nrom.prg_rom.len(), PRG_32K_BYTES);
    }

    #[test]
    fn test_nrom_from_prg_rom_truncate() {
        let prg_rom = vec![0; PRG_32K_BYTES + 1];
        let nrom = Nrom::from_prg_rom(prg_rom);
        assert_eq!(nrom.prg_rom.len(), PRG_32K_BYTES);
    }

    #[test]
    fn test_nrom_read_prg() {
        let mut prg_rom = vec![0; PRG_16K_BYTES];
        prg_rom[0] = 0xAA;
        let nrom = Nrom::from_prg_rom(prg_rom);

        // 0x8000 maps to offset 0
        assert_eq!(nrom.read_prg(0x8000), 0xAA);
        assert_eq!(<Nrom as Mapper>::read_prg(&nrom, 0x8000), 0xAA);
    }

    #[test]
    fn test_nrom_write_prg() {
        let mut nrom = Nrom::new_32k();
        // Should not panic, hardware ignores
        nrom.write_prg(0x8000, 0xAA);
        <Nrom as Mapper>::write_prg(&mut nrom, 0x8000, 0xAA);
    }

    #[test]
    fn test_nrom_from_prg_rom_partial_16k() {
        let prg_rom = vec![0; PRG_16K_BYTES - 1];
        let nrom = Nrom::from_prg_rom(prg_rom);
        assert_eq!(nrom.prg_rom.len(), PRG_16K_BYTES);
    }

    #[test]
    fn test_nrom_from_prg_rom_partial_32k() {
        let prg_rom = vec![0; PRG_16K_BYTES + 1];
        let nrom = Nrom::from_prg_rom(prg_rom);
        assert_eq!(nrom.prg_rom.len(), PRG_32K_BYTES);
    }
}
