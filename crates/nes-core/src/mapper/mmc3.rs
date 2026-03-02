use crate::rom::NametableMirroring;

use super::Mapper;

const PRG_BANK_8K: usize = 8 * 1024;
const CHR_BANK_1K: usize = 1024;
const CHR_WINDOW_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Mapper 4 (MMC3): banked PRG/CHR with scanline IRQ support.
pub struct Mmc3 {
    prg_bank_count_8k: u8,
    prg_rom: Vec<u8>,
    chr_bank_count_1k: u16,
    chr_data: Vec<u8>,
    chr_writable: bool,
    bank_select: u8,
    bank_registers: [u8; 8],
    mirroring: NametableMirroring,
    irq_latch: u8,
    irq_counter: u8,
    irq_reload: bool,
    irq_enabled: bool,
    irq_pending: bool,
}

impl Mmc3 {
    /// Creates a synthetic MMC3 mapper for tests.
    ///
    /// PRG banks are 8KB each and CHR banks are 1KB each. Every bank is filled
    /// with its bank index to make mapping assertions straightforward.
    #[must_use]
    pub fn new(prg_bank_count_8k: u8, chr_bank_count_1k: u16) -> Self {
        let prg_banks = prg_bank_count_8k.max(4);
        let chr_banks = chr_bank_count_1k.max(8);

        let mut prg_rom = vec![0_u8; prg_banks as usize * PRG_BANK_8K];
        for bank in 0..usize::from(prg_banks) {
            let start = bank * PRG_BANK_8K;
            let end = start + PRG_BANK_8K;
            prg_rom[start..end].fill(bank as u8);
        }

        let mut chr_data = vec![0_u8; chr_banks as usize * CHR_BANK_1K];
        for bank in 0..usize::from(chr_banks) {
            let start = bank * CHR_BANK_1K;
            let end = start + CHR_BANK_1K;
            chr_data[start..end].fill(bank as u8);
        }

        Self {
            prg_bank_count_8k: prg_banks,
            prg_rom,
            chr_bank_count_1k: chr_banks,
            chr_data,
            chr_writable: false,
            bank_select: 0,
            bank_registers: [0, 2, 4, 5, 0, 0, 0, 1],
            mirroring: NametableMirroring::Horizontal,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_pending: false,
        }
    }

    /// Builds MMC3 from raw ROM metadata.
    #[must_use]
    pub fn from_prg_chr(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mirroring: NametableMirroring) -> Self {
        let prg_bank_count_8k = (prg_rom.len() / PRG_BANK_8K) as u8;
        let (chr_data, chr_writable) = if chr_rom.is_empty() {
            (vec![0_u8; CHR_WINDOW_BYTES], true)
        } else {
            (chr_rom, false)
        };
        let chr_bank_count_1k = (chr_data.len() / CHR_BANK_1K) as u16;

        Self {
            prg_bank_count_8k: prg_bank_count_8k.max(4),
            prg_rom,
            chr_bank_count_1k: chr_bank_count_1k.max(8),
            chr_data,
            chr_writable,
            bank_select: 0,
            bank_registers: [0, 2, 4, 5, 0, 0, 0, 1],
            mirroring,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_pending: false,
        }
    }

    #[must_use]
    fn prg_mode(&self) -> bool {
        self.bank_select & 0x40 != 0
    }

    #[must_use]
    fn chr_inversion(&self) -> bool {
        self.bank_select & 0x80 != 0
    }

    #[must_use]
    fn last_prg_bank(&self) -> u8 {
        self.prg_bank_count_8k - 1
    }

    #[must_use]
    fn second_last_prg_bank(&self) -> u8 {
        self.last_prg_bank().saturating_sub(1)
    }

    #[must_use]
    fn normalize_prg_bank(&self, raw: u8) -> u8 {
        raw % self.prg_bank_count_8k
    }

    #[must_use]
    fn read_prg_bank(&self, bank: u8, addr: u16) -> u8 {
        let within = (usize::from(addr) - 0x8000) & 0x1FFF;
        let offset = usize::from(bank) * PRG_BANK_8K + within;
        self.prg_rom[offset]
    }

    #[must_use]
    fn prg_bank_for_slot(&self, slot: usize) -> u8 {
        let reg6 = self.normalize_prg_bank(self.bank_registers[6] & 0x3F);
        let reg7 = self.normalize_prg_bank(self.bank_registers[7] & 0x3F);
        let fixed_hi = self.second_last_prg_bank();
        let fixed_last = self.last_prg_bank();

        if self.prg_mode() {
            match slot {
                0 => fixed_hi,
                1 => reg7,
                2 => reg6,
                _ => fixed_last,
            }
        } else {
            match slot {
                0 => reg6,
                1 => reg7,
                2 => fixed_hi,
                _ => fixed_last,
            }
        }
    }

    #[must_use]
    fn normalize_chr_bank(&self, raw: u8) -> u16 {
        let count = self.chr_bank_count_1k.max(1);
        u16::from(raw) % count
    }

    fn copy_chr_1k_bank(&self, bank: u8, dst_offset: usize, dst: &mut [u8; CHR_WINDOW_BYTES]) {
        let index = usize::from(self.normalize_chr_bank(bank)) * CHR_BANK_1K;
        let end = index + CHR_BANK_1K;
        dst[dst_offset..dst_offset + CHR_BANK_1K].copy_from_slice(&self.chr_data[index..end]);
    }

    fn copy_chr_2k_bank(&self, bank: u8, dst_offset: usize, dst: &mut [u8; CHR_WINDOW_BYTES]) {
        let even = bank & !1;
        self.copy_chr_1k_bank(even, dst_offset, dst);
        self.copy_chr_1k_bank(even.wrapping_add(1), dst_offset + CHR_BANK_1K, dst);
    }

    /// Returns the currently mapped 8KB CHR window.
    #[must_use]
    pub fn chr_window(&self) -> [u8; CHR_WINDOW_BYTES] {
        let mut mapped = [0_u8; CHR_WINDOW_BYTES];
        if self.chr_inversion() {
            self.copy_chr_1k_bank(self.bank_registers[2], 0x0000, &mut mapped);
            self.copy_chr_1k_bank(self.bank_registers[3], 0x0400, &mut mapped);
            self.copy_chr_1k_bank(self.bank_registers[4], 0x0800, &mut mapped);
            self.copy_chr_1k_bank(self.bank_registers[5], 0x0C00, &mut mapped);
            self.copy_chr_2k_bank(self.bank_registers[0], 0x1000, &mut mapped);
            self.copy_chr_2k_bank(self.bank_registers[1], 0x1800, &mut mapped);
        } else {
            self.copy_chr_2k_bank(self.bank_registers[0], 0x0000, &mut mapped);
            self.copy_chr_2k_bank(self.bank_registers[1], 0x0800, &mut mapped);
            self.copy_chr_1k_bank(self.bank_registers[2], 0x1000, &mut mapped);
            self.copy_chr_1k_bank(self.bank_registers[3], 0x1400, &mut mapped);
            self.copy_chr_1k_bank(self.bank_registers[4], 0x1800, &mut mapped);
            self.copy_chr_1k_bank(self.bank_registers[5], 0x1C00, &mut mapped);
        }
        mapped
    }

    /// Returns `true` when mapped CHR should be writable by the PPU.
    #[must_use]
    pub fn chr_writable(&self) -> bool {
        self.chr_writable
    }

    /// Returns current nametable mirroring mode controlled by MMC3.
    #[must_use]
    pub fn mirroring(&self) -> NametableMirroring {
        self.mirroring
    }

    /// Returns pending mapper IRQ level.
    #[must_use]
    pub fn irq_pending(&self) -> bool {
        self.irq_pending
    }

    /// Advances scanline IRQ logic based on current PPU dot.
    pub fn on_ppu_dot(&mut self, scanline: u16, dot: u16, rendering_enabled: bool) {
        if !rendering_enabled || dot != 260 {
            return;
        }
        if scanline >= 240 && scanline != 261 {
            return;
        }

        if self.irq_counter == 0 || self.irq_reload {
            self.irq_counter = self.irq_latch;
            self.irq_reload = false;
        } else {
            self.irq_counter = self.irq_counter.saturating_sub(1);
        }

        if self.irq_counter == 0 && self.irq_enabled {
            self.irq_pending = true;
        }
    }

    /// Reads PRG using MMC3 bank mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Writes to MMC3 register space.
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }
}

impl Mapper for Mmc3 {
    fn read_prg(&self, addr: u16) -> u8 {
        let slot = ((usize::from(addr) - 0x8000) / PRG_BANK_8K).min(3);
        let bank = self.prg_bank_for_slot(slot);
        self.read_prg_bank(bank, addr)
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        if addr < 0x8000 {
            return;
        }
        match addr {
            0x8000..=0x9FFE if addr & 1 == 0 => {
                self.bank_select = value;
            }
            0x8001..=0x9FFF if addr & 1 == 1 => {
                let register = (self.bank_select & 0x07) as usize;
                self.bank_registers[register] = match register {
                    0 | 1 => value & 0xFE,
                    6 | 7 => value & 0x3F,
                    _ => value,
                };
            }
            0xA000..=0xBFFE if addr & 1 == 0 => {
                self.mirroring = if value & 1 == 0 {
                    NametableMirroring::Vertical
                } else {
                    NametableMirroring::Horizontal
                };
            }
            0xA001..=0xBFFF if addr & 1 == 1 => {
                // PRG RAM protect is currently ignored.
            }
            0xC000..=0xDFFE if addr & 1 == 0 => {
                self.irq_latch = value;
            }
            0xC001..=0xDFFF if addr & 1 == 1 => {
                self.irq_reload = true;
            }
            0xE000..=0xFFFE if addr & 1 == 0 => {
                self.irq_enabled = false;
                self.irq_pending = false;
            }
            0xE001..=0xFFFF if addr & 1 == 1 => {
                self.irq_enabled = true;
            }
            _ => {}
        }
    }
}
