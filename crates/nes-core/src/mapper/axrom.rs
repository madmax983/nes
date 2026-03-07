use crate::rom::NametableMirroring;

use super::Mapper;

const PRG_BANK_32K: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Mapper 7 (AxROM): switchable 32KB PRG bank with one-screen mirroring control.
pub struct Axrom {
    bank_count: u8,
    selected_bank: u8,
    selected_nametable_bank: u8,
    prg_rom: Vec<u8>,
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
