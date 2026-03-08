use super::Mapper;

const PRG_BANK_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Mapper 2 (UxROM): switchable lower 16K bank + fixed upper 16K bank.
pub struct Uxrom {
    bank_count: usize,
    selected_bank: usize,
    prg_rom: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
