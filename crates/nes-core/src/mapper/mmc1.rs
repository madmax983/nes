use super::Mapper;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mmc1 {
    prg_bank_count: u8,
    _chr_bank_count: u8,
    shift_register: u8,
    shift_count: u8,
    selected_prg_bank: u8,
    prg_rom: Vec<u8>,
}

impl Mmc1 {
    const SHIFT_RESET: u8 = 0x10;

    #[must_use]
    pub fn new(prg_bank_count: u8, chr_bank_count: u8) -> Self {
        let effective_prg_banks = prg_bank_count.max(1);
        let prg_rom = vec![0_u8; effective_prg_banks as usize * 16 * 1024];

        Self {
            prg_bank_count: effective_prg_banks,
            _chr_bank_count: chr_bank_count.max(1),
            shift_register: Self::SHIFT_RESET,
            shift_count: 0,
            selected_prg_bank: 0,
            prg_rom,
        }
    }

    #[must_use]
    pub fn from_prg_rom(prg_rom: Vec<u8>, chr_bank_count: u8) -> Self {
        let prg_bank_count = (prg_rom.len() / (16 * 1024)) as u8;
        Self {
            prg_bank_count: prg_bank_count.max(1),
            _chr_bank_count: chr_bank_count.max(1),
            shift_register: Self::SHIFT_RESET,
            shift_count: 0,
            selected_prg_bank: 0,
            prg_rom,
        }
    }

    #[must_use]
    pub fn shift_is_reset(&self) -> bool {
        self.shift_register == Self::SHIFT_RESET && self.shift_count == 0
    }

    #[must_use]
    pub fn selected_prg_bank(&self) -> u8 {
        self.selected_prg_bank
    }

    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }

    fn reset_shift(&mut self) {
        self.shift_register = Self::SHIFT_RESET;
        self.shift_count = 0;
    }

    fn commit_shift_register(&mut self, addr: u16) {
        if addr >= 0xE000 {
            self.selected_prg_bank = self.shift_register % self.prg_bank_count;
        }
        self.reset_shift();
    }

    fn push_shift_bit(&mut self, value: u8) {
        let incoming = value & 1;
        self.shift_register = (self.shift_register >> 1) | (incoming << 4);
        self.shift_count = self.shift_count.saturating_add(1);
    }

    fn bank_offset(&self, bank: u8) -> usize {
        bank as usize * 16 * 1024
    }

    fn read_bank(&self, bank: u8, addr: u16) -> u8 {
        let within_bank = (addr as usize) & 0x3FFF;
        self.prg_rom[self.bank_offset(bank) + within_bank]
    }
}

impl Mapper for Mmc1 {
    fn read_prg(&self, addr: u16) -> u8 {
        if addr < 0xC000 {
            self.read_bank(self.selected_prg_bank, addr)
        } else {
            self.read_bank(self.prg_bank_count - 1, addr)
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        if value & 0x80 != 0 {
            self.reset_shift();
            return;
        }

        self.push_shift_bit(value);
        if self.shift_count >= 5 {
            self.commit_shift_register(addr);
        }
    }
}
