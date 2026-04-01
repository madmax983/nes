use super::Mapper;
use serde::{Deserialize, Serialize};

const PRG_BANK_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Mapper 2 (UxROM): switchable lower 16K bank + fixed upper 16K bank.
pub struct Uxrom {
    bank_count: usize,
    selected_bank: usize,
    prg_rom: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UxromState {
    pub selected_bank: u8,
}

impl Uxrom {
    /// Creates a synthetic mapper instance with the requested PRG bank count.
    #[must_use]
    pub fn new(bank_count: u8) -> Self {
        let effective_bank_count = usize::from(bank_count.max(1));
        let mut prg_rom = vec![0_u8; effective_bank_count * PRG_BANK_BYTES];
        for (idx, byte) in prg_rom.iter_mut().enumerate() {
            *byte = (idx & 0xFF) as u8;
        }

        Self {
            bank_count: effective_bank_count,
            selected_bank: 0,
            prg_rom,
        }
    }

    /// Builds UxROM from raw PRG ROM bytes.
    #[must_use]
    pub fn from_prg_rom(mut prg_rom: Vec<u8>) -> Self {
        if prg_rom.is_empty() {
            prg_rom.resize(PRG_BANK_BYTES, 0);
        }
        let remainder = prg_rom.len() % PRG_BANK_BYTES;
        if remainder != 0 {
            prg_rom.resize(prg_rom.len() + (PRG_BANK_BYTES - remainder), 0);
        }
        let bank_count = (prg_rom.len() / PRG_BANK_BYTES).max(1);
        Self {
            bank_count,
            selected_bank: 0,
            prg_rom,
        }
    }

    /// Returns the currently selected switchable bank.
    #[must_use]
    pub fn selected_bank(&self) -> u8 {
        self.selected_bank as u8
    }

    #[must_use]
    pub(crate) fn state(&self) -> UxromState {
        UxromState {
            selected_bank: self.selected_bank as u8,
        }
    }

    pub(crate) fn restore_state(&mut self, state: UxromState) {
        self.selected_bank = usize::from(state.selected_bank) % self.bank_count;
    }

    /// Reads PRG byte through UxROM bank mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Applies PRG-space write as bank-select register update.
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }

    fn bank_offset(&self, bank: usize) -> usize {
        bank * PRG_BANK_BYTES
    }

    fn last_bank(&self) -> usize {
        self.bank_count - 1
    }

    fn read_bank(&self, bank: usize, addr: u16) -> u8 {
        let within_bank = (addr as usize) & 0x3FFF;
        self.prg_rom[self.bank_offset(bank) + within_bank]
    }
}

impl Mapper for Uxrom {
    fn read_prg(&self, addr: u16) -> u8 {
        if addr < 0xC000 {
            self.read_bank(self.selected_bank, addr)
        } else {
            self.read_bank(self.last_bank(), addr)
        }
    }

    fn write_prg(&mut self, _addr: u16, value: u8) {
        self.selected_bank = usize::from(value) % self.bank_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uxrom_state_can_be_restored() {
        let mut mapper = Uxrom::new(2);
        mapper.write_prg(0x8000, 1);
        let state = mapper.state();
        let mut restored = Uxrom::new(2);
        restored.restore_state(state);
        assert_eq!(mapper.selected_bank(), restored.selected_bank());
    }

    #[test]
    fn uxrom_read_prg_resolves_via_mapper_trait() {
        let mapper = Uxrom::new(2);
        assert_eq!(mapper.read_prg(0x8000), 0);
    }

    #[test]
    fn uxrom_from_prg_rom_pads_partial_bank() {
        // Test padding logic: `prg_rom.resize(prg_rom.len() + (PRG_BANK_BYTES - remainder), 0);`
        // 16KB + 1 byte remainder -> should pad to 32KB
        let prg_rom = vec![0; PRG_BANK_BYTES + 1];
        let mapper = Uxrom::from_prg_rom(prg_rom);

        // bank_count should be (16385 / 16384).max(1) which is 1 normally, but with padding it's 32768 / 16384 = 2.
        assert_eq!(mapper.bank_count, 2);
        assert_eq!(mapper.prg_rom.len(), 2 * PRG_BANK_BYTES);
    }

    #[test]
    fn uxrom_from_prg_rom_pads_empty_prg() {
        let mapper = Uxrom::from_prg_rom(vec![]);
        assert_eq!(mapper.bank_count, 1);
        assert_eq!(mapper.prg_rom.len(), PRG_BANK_BYTES);
    }
}
