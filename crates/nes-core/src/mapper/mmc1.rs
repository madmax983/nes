use super::Mapper;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Mapper 1 (MMC1) PRG banking subset used by the core.
///
/// This implementation focuses on the 5-bit serial shift register protocol
/// and PRG banking behavior needed for current compatibility targets.
pub struct Mmc1 {
    prg_bank_count: u8,
    _chr_bank_count: u8,
    control: u8,
    shift_register: u8,
    shift_count: u8,
    selected_prg_bank: u8,
    prg_rom: Vec<u8>,
}

impl Mmc1 {
    const SHIFT_RESET: u8 = 0x10;
    const CONTROL_RESET: u8 = 0x0C;

    /// Creates a synthetic MMC1 instance for tests.
    #[must_use]
    pub fn new(prg_bank_count: u8, chr_bank_count: u8) -> Self {
        let effective_prg_banks = prg_bank_count.max(1);
        let prg_rom = vec![0_u8; effective_prg_banks as usize * 16 * 1024];

        Self {
            prg_bank_count: effective_prg_banks,
            _chr_bank_count: chr_bank_count.max(1),
            control: Self::CONTROL_RESET,
            shift_register: Self::SHIFT_RESET,
            shift_count: 0,
            selected_prg_bank: 0,
            prg_rom,
        }
    }

    /// Builds MMC1 from PRG ROM bytes and CHR bank metadata.
    #[must_use]
    pub fn from_prg_rom(prg_rom: Vec<u8>, chr_bank_count: u8) -> Self {
        let prg_bank_count = (prg_rom.len() / (16 * 1024)) as u8;
        Self {
            prg_bank_count: prg_bank_count.max(1),
            _chr_bank_count: chr_bank_count.max(1),
            control: Self::CONTROL_RESET,
            shift_register: Self::SHIFT_RESET,
            shift_count: 0,
            selected_prg_bank: 0,
            prg_rom,
        }
    }

    /// Returns whether the serial shift register is in reset state.
    #[must_use]
    pub fn shift_is_reset(&self) -> bool {
        self.shift_register == Self::SHIFT_RESET && self.shift_count == 0
    }

    /// Returns selected PRG bank register value.
    #[must_use]
    pub fn selected_prg_bank(&self) -> u8 {
        self.selected_prg_bank
    }

    /// Reads PRG through current MMC1 bank mode.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Writes serial control bits into MMC1 mapper registers.
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }

    fn reset_shift(&mut self) {
        self.shift_register = Self::SHIFT_RESET;
        self.shift_count = 0;
    }

    fn commit_shift_register(&mut self, addr: u16) {
        let value = self.shift_register & 0x1F;
        match addr {
            0x8000..=0x9FFF => {
                self.control = value;
            }
            0xE000..=0xFFFF => {
                self.selected_prg_bank = value & 0x0F;
            }
            _ => {}
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
        let bank_count = self.prg_bank_count.max(1);
        let prg_mode = (self.control >> 2) & 0b11;
        let selected = self.selected_prg_bank % bank_count;

        let bank = match prg_mode {
            0 | 1 => {
                let lower = (selected & 0xFE) % bank_count;
                if addr < 0xC000 {
                    lower
                } else {
                    lower.wrapping_add(1) % bank_count
                }
            }
            2 => {
                if addr < 0xC000 {
                    0
                } else {
                    selected
                }
            }
            _ => {
                if addr < 0xC000 {
                    selected
                } else {
                    bank_count - 1
                }
            }
        };

        self.read_bank(bank, addr)
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        if value & 0x80 != 0 {
            self.reset_shift();
            self.control |= Self::CONTROL_RESET;
            return;
        }

        self.push_shift_bit(value);
        if self.shift_count >= 5 {
            self.commit_shift_register(addr);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mmc1_reset_shift_clears_count_and_sets_bit4() {
        let mut m = Mmc1::new(16, 8);
        m.write_prg(0x8000, 0x01); // shift bit
        assert!(!m.shift_is_reset());
        m.write_prg(0x8000, 0x80); // reset
        assert!(m.shift_is_reset());
    }

    #[test]
    fn mmc1_shift_register_accumulates_bits_and_commits_on_fifth_write() {
        let mut m = Mmc1::new(16, 8);
        m.write_prg(0x8000, 0x01); // 1
        m.write_prg(0x8000, 0x00); // 0
        m.write_prg(0x8000, 0x01); // 1
        m.write_prg(0x8000, 0x00); // 0
        m.write_prg(0x8000, 0x01); // 1, now commit to 0x8000 (control)
        // Shift bits enter at MSB (bit 4) and shift right.
        // Sequence: 1, 0, 1, 0, 1 -> 10101 binary -> 0x15
        assert_eq!(m.control, 0x15);
        assert!(m.shift_is_reset());
    }

    #[test]
    fn mmc1_commit_to_prg_bank_updates_selected_bank() {
        let mut m = Mmc1::new(16, 8);
        // We will commit a value to E000-FFFF. Let's write binary 01010 (0x0A)
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x01);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x01);
        m.write_prg(0xE000, 0x00);
        // LSB is written first.
        // bit0=0, bit1=1, bit2=0, bit3=1, bit4=0 -> 0x0A
        assert_eq!(m.selected_prg_bank(), 0x0A);
    }

    #[test]
    fn mmc1_prg_mode_0_and_1_switch_32kb_banks() {
        let mut m = Mmc1::new(4, 8); // 4 PRG banks (64KB total)
        // Set first byte of bank 0
        m.prg_rom[0] = 0x11;
        // Set first byte of bank 1 (0x4000)
        m.prg_rom[0x4000] = 0x22;
        // Set first byte of bank 2 (0x8000)
        m.prg_rom[0x8000] = 0x33;
        // Set first byte of bank 3 (0xC000)
        m.prg_rom[0xC000] = 0x44;

        // Write control register (0x8000) to set PRG mode 0 (bits 2,3 = 0)
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x00);

        // Select bank 2 (0xE000-FFFF register)
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x01); // bit 1 = 1
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00); // 00010 binary -> 2

        // Mode 0/1 uses even/odd banks. It ignores the lowest bit of the selected bank.
        // Selected = 2 -> 0x8000 uses bank 2, 0xC000 uses bank 3
        assert_eq!(m.read_prg(0x8000), 0x33);
        assert_eq!(m.read_prg(0xC000), 0x44);

        // Select bank 3 (0xE000-FFFF register)
        m.write_prg(0xE000, 0x01); // bit 0 = 1
        m.write_prg(0xE000, 0x01); // bit 1 = 1
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00); // 00011 binary -> 3

        // Mode 0/1 ignores lowest bit, so 3 -> banks 2 and 3
        assert_eq!(m.read_prg(0x8000), 0x33);
        assert_eq!(m.read_prg(0xC000), 0x44);

        // Write control register to set PRG mode 1 (bits 2,3 = 0,1)
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x01);
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x00);

        assert_eq!(m.read_prg(0x8000), 0x33);
        assert_eq!(m.read_prg(0xC000), 0x44);
    }

    #[test]
    fn mmc1_prg_mode_2_fixes_first_bank_and_switches_second() {
        let mut m = Mmc1::new(4, 8); // 4 PRG banks (64KB total)
        m.prg_rom[0] = 0x11; // Bank 0
        m.prg_rom[0x4000] = 0x22; // Bank 1
        m.prg_rom[0x8000] = 0x33; // Bank 2
        m.prg_rom[0xC000] = 0x44; // Bank 3

        // Write control register to set PRG mode 2 (bits 2,3 = 1,0)
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x01);
        m.write_prg(0x8000, 0x00);

        // Select bank 2
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x01);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00); // 00010 -> 2

        // Mode 2 fixes 0x8000 at bank 0, switches 0xC000
        assert_eq!(m.read_prg(0x8000), 0x11);
        assert_eq!(m.read_prg(0xC000), 0x33);

        // Select bank 3
        m.write_prg(0xE000, 0x01);
        m.write_prg(0xE000, 0x01);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00); // 00011 -> 3

        assert_eq!(m.read_prg(0x8000), 0x11);
        assert_eq!(m.read_prg(0xC000), 0x44);
    }

    #[test]
    fn mmc1_prg_mode_3_switches_first_bank_and_fixes_last() {
        let mut m = Mmc1::new(4, 8); // 4 PRG banks (64KB total)
        m.prg_rom[0] = 0x11; // Bank 0
        m.prg_rom[0x4000] = 0x22; // Bank 1
        m.prg_rom[0x8000] = 0x33; // Bank 2
        m.prg_rom[0xC000] = 0x44; // Bank 3

        // Write control register to set PRG mode 3 (bits 2,3 = 1,1)
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x00);
        m.write_prg(0x8000, 0x01);
        m.write_prg(0x8000, 0x01);
        m.write_prg(0x8000, 0x00);

        // Select bank 1
        m.write_prg(0xE000, 0x01);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00); // 00001 -> 1

        // Mode 3 switches 0x8000, fixes 0xC000 at last bank (bank 3)
        assert_eq!(m.read_prg(0x8000), 0x22);
        assert_eq!(m.read_prg(0xC000), 0x44);

        // Select bank 2
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x01);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00);
        m.write_prg(0xE000, 0x00); // 00010 -> 2

        assert_eq!(m.read_prg(0x8000), 0x33);
        assert_eq!(m.read_prg(0xC000), 0x44);
    }

    #[test]
    fn mmc1_commit_to_unhandled_range_does_not_panic() {
        let mut m = Mmc1::new(16, 8);
        // Address 0xA000 is CHR bank 0, unhandled in our current simplified mmc1
        m.write_prg(0xA000, 0x01);
        m.write_prg(0xA000, 0x01);
        m.write_prg(0xA000, 0x01);
        m.write_prg(0xA000, 0x01);
        m.write_prg(0xA000, 0x01);
        assert!(m.shift_is_reset());

        // Address 0xC000 is CHR bank 1, unhandled in our current simplified mmc1
        m.write_prg(0xC000, 0x01);
        m.write_prg(0xC000, 0x01);
        m.write_prg(0xC000, 0x01);
        m.write_prg(0xC000, 0x01);
        m.write_prg(0xC000, 0x01);
        assert!(m.shift_is_reset());
    }
}
