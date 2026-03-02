use super::Mapper;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Mapper 2 (UxROM): switchable lower 16K bank + fixed upper 16K bank.
pub struct Uxrom {
    bank_count: u8,
    selected_bank: u8,
    prg_rom: Vec<u8>,
}

impl Uxrom {
    /// Creates a synthetic mapper instance with the requested PRG bank count.
    #[must_use]
    pub fn new(bank_count: u8) -> Self {
        let effective_bank_count = bank_count.max(1);
        let mut prg_rom = vec![0_u8; effective_bank_count as usize * 16 * 1024];
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
    pub fn from_prg_rom(prg_rom: Vec<u8>) -> Self {
        let bank_count = (prg_rom.len() / (16 * 1024)) as u8;
        Self {
            bank_count: bank_count.max(1),
            selected_bank: 0,
            prg_rom,
        }
    }

    /// Returns the currently selected switchable bank.
    #[must_use]
    pub fn selected_bank(&self) -> u8 {
        self.selected_bank
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

    fn bank_offset(&self, bank: u8) -> usize {
        bank as usize * 16 * 1024
    }

    fn last_bank(&self) -> u8 {
        self.bank_count - 1
    }

    fn read_bank(&self, bank: u8, addr: u16) -> u8 {
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
        self.selected_bank = value % self.bank_count;
    }
}
