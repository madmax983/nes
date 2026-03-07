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
    pub fn from_prg_chr(
        mut prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        mirroring: NametableMirroring,
    ) -> Self {
        let min_prg_bytes = 4 * PRG_BANK_8K;
        if prg_rom.len() < min_prg_bytes {
            prg_rom.resize(min_prg_bytes, 0);
        }
        let prg_remainder = prg_rom.len() % PRG_BANK_8K;
        if prg_remainder != 0 {
            prg_rom.resize(prg_rom.len() + (PRG_BANK_8K - prg_remainder), 0);
        }
        let prg_bank_count_8k = (prg_rom.len() / PRG_BANK_8K) as u8;

        let (mut chr_data, chr_writable) = if chr_rom.is_empty() {
            (vec![0_u8; CHR_WINDOW_BYTES], true)
        } else {
            (chr_rom, false)
        };
        if chr_data.len() < CHR_WINDOW_BYTES {
            chr_data.resize(CHR_WINDOW_BYTES, 0);
        }
        let chr_remainder = chr_data.len() % CHR_BANK_1K;
        if chr_remainder != 0 {
            chr_data.resize(chr_data.len() + (CHR_BANK_1K - chr_remainder), 0);
        }
        let chr_bank_count_1k = (chr_data.len() / CHR_BANK_1K) as u16;

        Self {
            prg_bank_count_8k,
            prg_rom,
            chr_bank_count_1k,
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

    fn queue_chr_1k_update(
        &self,
        bank: u8,
        src_offset: usize,
        window: &[u8; CHR_WINDOW_BYTES],
        planned_offsets: &mut [Option<usize>],
    ) {
        let bank_index = usize::from(self.normalize_chr_bank(bank));
        let dst_start = bank_index * CHR_BANK_1K;
        let dst_end = dst_start + CHR_BANK_1K;
        let src_slice = &window[src_offset..src_offset + CHR_BANK_1K];
        if src_slice != &self.chr_data[dst_start..dst_end] {
            planned_offsets[bank_index] = Some(src_offset);
        }
    }

    fn queue_chr_2k_update(
        &self,
        bank: u8,
        src_offset: usize,
        window: &[u8; CHR_WINDOW_BYTES],
        planned_offsets: &mut [Option<usize>],
    ) {
        let even = bank & !1;
        self.queue_chr_1k_update(even, src_offset, window, planned_offsets);
        self.queue_chr_1k_update(
            even.wrapping_add(1),
            src_offset + CHR_BANK_1K,
            window,
            planned_offsets,
        );
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

    /// Synchronizes writable CHR-RAM from the currently visible PPU CHR window.
    pub fn sync_chr_ram_from_ppu_window(&mut self, window: &[u8; CHR_WINDOW_BYTES]) {
        if !self.chr_writable {
            return;
        }

        let mut planned_offsets = vec![None; usize::from(self.chr_bank_count_1k.max(1))];

        if self.chr_inversion() {
            self.queue_chr_1k_update(self.bank_registers[2], 0x0000, window, &mut planned_offsets);
            self.queue_chr_1k_update(self.bank_registers[3], 0x0400, window, &mut planned_offsets);
            self.queue_chr_1k_update(self.bank_registers[4], 0x0800, window, &mut planned_offsets);
            self.queue_chr_1k_update(self.bank_registers[5], 0x0C00, window, &mut planned_offsets);
            self.queue_chr_2k_update(self.bank_registers[0], 0x1000, window, &mut planned_offsets);
            self.queue_chr_2k_update(self.bank_registers[1], 0x1800, window, &mut planned_offsets);
        } else {
            self.queue_chr_2k_update(self.bank_registers[0], 0x0000, window, &mut planned_offsets);
            self.queue_chr_2k_update(self.bank_registers[1], 0x0800, window, &mut planned_offsets);
            self.queue_chr_1k_update(self.bank_registers[2], 0x1000, window, &mut planned_offsets);
            self.queue_chr_1k_update(self.bank_registers[3], 0x1400, window, &mut planned_offsets);
            self.queue_chr_1k_update(self.bank_registers[4], 0x1800, window, &mut planned_offsets);
            self.queue_chr_1k_update(self.bank_registers[5], 0x1C00, window, &mut planned_offsets);
        }

        for (bank_index, src_offset) in planned_offsets.into_iter().enumerate() {
            if let Some(src_offset) = src_offset {
                let dst_start = bank_index * CHR_BANK_1K;
                let dst_end = dst_start + CHR_BANK_1K;
                self.chr_data[dst_start..dst_end]
                    .copy_from_slice(&window[src_offset..src_offset + CHR_BANK_1K]);
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_prg_chr_short_inputs_do_not_panic_on_reads() {
        let m = Mmc3::from_prg_chr(
            vec![0x11; PRG_BANK_8K + 17],
            vec![0x22; (4 * CHR_BANK_1K) + 9],
            NametableMirroring::Horizontal,
        );

        let _ = m.read_prg(0x8000);
        let _ = m.read_prg(0xE000);
        let window = m.chr_window();
        assert_eq!(window.len(), CHR_WINDOW_BYTES);
    }
}

#[test]
fn sync_chr_ram_from_ppu_window_updates_chr_data_for_writable_banks() {
    let mut m = Mmc3::from_prg_chr(
        vec![0x11; 4 * PRG_BANK_8K],
        vec![], // Empty CHR ROM makes it CHR RAM
        NametableMirroring::Horizontal,
    );

    assert!(m.chr_writable());

    // Default layout:
    // Bank 0 (2k) at 0x0000
    // Bank 1 (2k) at 0x0800
    // Bank 2 (1k) at 0x1000
    // Bank 3 (1k) at 0x1400
    // Bank 4 (1k) at 0x1800
    // Bank 5 (1k) at 0x1C00
    // Initial registers for these are mostly 0 or basic values from constructor.

    // Grab current window, mutate it to simulate PPU write
    let mut window = m.chr_window();

    // Write to bank 2 mapped at 0x1000
    window[0x1000] = 0xAA;
    window[0x1001] = 0xBB;

    // Write to bank 0 mapped at 0x0000 (which is a 2K bank split into two 1K banks)
    window[0x0000] = 0xCC;

    m.sync_chr_ram_from_ppu_window(&window);

    // Since m.bank_registers[2] maps to CHR bank 4 according to new constructor (0, 2, 4, 5, 0, 0, 0, 1)
    // Wait, let's just re-read the window and see if the writes persisted
    let window2 = m.chr_window();
    assert_eq!(window2[0x1000], 0xAA);
    assert_eq!(window2[0x1001], 0xBB);
    assert_eq!(window2[0x0000], 0xCC);

    // Change inversion to hit the other branch
    m.write_prg(0x8000, 0x80); // Set bit 7 for CHR inversion
    let mut window3 = m.chr_window();
    // Now Bank 0 (2k) is at 0x1000. Write something there
    window3[0x1000] = 0xDD;
    m.sync_chr_ram_from_ppu_window(&window3);

    let window4 = m.chr_window();
    assert_eq!(window4[0x1000], 0xDD);
}
