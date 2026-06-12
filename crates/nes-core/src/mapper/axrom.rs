use crate::rom::NametableMirroring;
use serde::{Deserialize, Serialize};

use super::Mapper;

const PRG_BANK_32K: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Mapper 7 (AxROM): switchable 32KB PRG bank with one-screen mirroring control.
pub struct Axrom {
    bank_count: u8,
    selected_bank: u8,
    selected_nametable_bank: u8,
    prg_rom: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AxromState {
    pub selected_bank: u8,
    pub selected_nametable_bank: u8,
}

impl Axrom {
    /// Builds AxROM from raw PRG ROM bytes.
    ///
    /// If fewer than 32KB are provided, the bank is zero-padded so reads stay
    /// in-bounds across the full `$8000..=$FFFF` PRG window.
    #[must_use]
    pub fn from_prg_rom(mut prg_rom: Vec<u8>) -> Self {
        if prg_rom.len() < PRG_BANK_32K {
            prg_rom.resize(PRG_BANK_32K, 0);
        }
        let bank_count = (prg_rom.len() / PRG_BANK_32K) as u8;
        Self {
            bank_count: bank_count.max(1),
            selected_bank: 0,
            selected_nametable_bank: 0,
            prg_rom,
        }
    }

    /// Returns the currently selected 32KB PRG bank.
    #[must_use]
    pub fn selected_bank(&self) -> u8 {
        self.selected_bank
    }

    /// Returns the selected nametable page for one-screen mirroring.
    ///
    /// `0` maps all nametables to the first page, `1` to the second.
    #[must_use]
    pub fn selected_nametable_bank(&self) -> u8 {
        self.selected_nametable_bank
    }

    #[must_use]
    pub(crate) fn state(&self) -> AxromState {
        AxromState {
            selected_bank: self.selected_bank,
            selected_nametable_bank: self.selected_nametable_bank,
        }
    }

    pub(crate) fn restore_state(&mut self, state: AxromState) {
        self.selected_bank = state.selected_bank % self.bank_count;
        self.selected_nametable_bank = state.selected_nametable_bank & 0x01;
    }

    /// Returns the active one-screen nametable mirroring mode.
    #[must_use]
    pub fn mirroring(&self) -> NametableMirroring {
        if self.selected_nametable_bank == 0 {
            NametableMirroring::OneScreenLower
        } else {
            NametableMirroring::OneScreenUpper
        }
    }

    /// Reads PRG byte through current AxROM 32KB mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }
    /// Applies AxROM bank/mirroring register writes.
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }

    fn bank_offset(&self, bank: u8) -> usize {
        usize::from(bank) * PRG_BANK_32K
    }
}

impl Mapper for Axrom {
    fn read_prg(&self, addr: u16) -> u8 {
        let within_bank = usize::from(addr) & 0x7FFF;
        let offset = self.bank_offset(self.selected_bank) + within_bank;
        self.prg_rom[offset]
    }

    fn write_prg(&mut self, _addr: u16, value: u8) {
        self.selected_bank = (value & 0x07) % self.bank_count;
        self.selected_nametable_bank = (value >> 4) & 0x01;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axrom_state_can_be_restored() {
        let prg = vec![0_u8; 64 * 1024];
        let mut mapper = Axrom::from_prg_rom(prg);

        mapper.write_prg(0x8000, 0x11);

        let state = mapper.state();

        let mut mapper_restored = Axrom::from_prg_rom(vec![0_u8; 64 * 1024]);
        mapper_restored.restore_state(state);

        assert_eq!(mapper.selected_bank(), mapper_restored.selected_bank());
        assert_eq!(
            mapper.selected_nametable_bank(),
            mapper_restored.selected_nametable_bank()
        );
    }

    #[test]
    fn axrom_mirroring_reflects_selected_nametable_bank() {
        let mut mapper = Axrom::from_prg_rom(vec![0_u8; 32 * 1024]);

        mapper.write_prg(0x8000, 0x00);
        assert_eq!(mapper.mirroring(), NametableMirroring::OneScreenLower);

        mapper.write_prg(0x8000, 0x10);
        assert_eq!(mapper.mirroring(), NametableMirroring::OneScreenUpper);
    }

    #[test]
    fn axrom_from_prg_rom_pads_short_rom() {
        let prg = vec![0_u8; 16 * 1024];
        let mapper = Axrom::from_prg_rom(prg);
        assert_eq!(mapper.prg_rom.len(), 32 * 1024);
    }

    #[test]
    fn axrom_read_and_write_via_methods_calls_trait() {
        let mut mapper = Axrom::from_prg_rom(vec![0_u8; 64 * 1024]);
        mapper.prg_rom[32 * 1024] = 0xAA; // Start of bank 1

        mapper.write_prg(0x8000, 0x01); // Switch to bank 1
        assert_eq!(mapper.selected_bank(), 1);

        assert_eq!(mapper.read_prg(0x8000), 0xAA);
    }
}
