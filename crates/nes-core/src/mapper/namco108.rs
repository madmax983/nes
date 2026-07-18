use super::Mapper;
use serde::{Deserialize, Serialize};

const PRG_BANK_8K: usize = 8 * 1024;
const CHR_BANK_1K: usize = 1024;
const CHR_WINDOW_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Mapper 206 (Namco-108 / DxROM): MMC3-style banking without the PRG mode bit,
/// CHR inversion, scanline IRQ, mirroring register, or PRG-RAM.
///
/// `$8000` (even) selects a register index (bits 0-2); `$8001` (odd) writes the
/// selected register. Register values are masked to 6 bits. R0/R1 are 2KB CHR
/// banks (bit 0 ignored), R2-R5 are 1KB CHR banks, and R6/R7 are 8KB PRG banks.
/// The layout is fixed (MMC3 mode 0, no inversion): PRG `$8000`/`$A000` follow
/// R6/R7 while `$C000`/`$E000` are fixed to the last two banks.
pub struct Namco108 {
    prg_bank_count_8k: usize,
    prg_rom: Vec<u8>,
    chr_bank_count_1k: usize,
    chr_data: Vec<u8>,
    chr_writable: bool,
    bank_select: u8,
    bank_registers: [u8; 8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Namco108State {
    pub bank_select: u8,
    pub bank_registers: [u8; 8],
}

impl Namco108 {
    /// Builds Namco-108 from raw PRG/CHR data.
    ///
    /// Inputs are normalized to avoid panics:
    /// - PRG is zero-padded to at least four 8KB banks and rounded up to a full bank.
    /// - Empty CHR becomes one writable 8KB CHR-RAM window; non-empty CHR is
    ///   padded to a full 1KB bank.
    #[must_use]
    pub fn from_prg_chr(mut prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        let min_prg_bytes = 4 * PRG_BANK_8K;
        if prg_rom.len() < min_prg_bytes {
            prg_rom.resize(min_prg_bytes, 0);
        }
        let prg_remainder = prg_rom.len() % PRG_BANK_8K;
        if prg_remainder != 0 {
            prg_rom.resize(prg_rom.len() + (PRG_BANK_8K - prg_remainder), 0);
        }
        let prg_bank_count_8k = prg_rom.len() / PRG_BANK_8K;

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
        let chr_bank_count_1k = chr_data.len() / CHR_BANK_1K;

        Self {
            prg_bank_count_8k,
            prg_rom,
            chr_bank_count_1k,
            chr_data,
            chr_writable,
            bank_select: 0,
            bank_registers: [0, 2, 4, 5, 6, 7, 0, 1],
        }
    }

    #[must_use]
    pub(crate) fn state(&self) -> Namco108State {
        Namco108State {
            bank_select: self.bank_select,
            bank_registers: self.bank_registers,
        }
    }

    /// Restores state from a snapshot.
    ///
    /// Performance note: Takes state by reference to avoid heap allocations
    /// when restoring structs with large dynamically allocated fields.
    pub(crate) fn restore_state(&mut self, state: &Namco108State) {
        self.bank_select = state.bank_select;
        self.bank_registers = state.bank_registers;
    }

    fn normalize_prg_bank(&self, raw: u8) -> usize {
        usize::from(raw) % self.prg_bank_count_8k
    }

    fn normalize_chr_bank(&self, raw: u8) -> usize {
        usize::from(raw) % self.chr_bank_count_1k.max(1)
    }

    fn prg_bank_for_slot(&self, slot: usize) -> usize {
        match slot {
            0 => self.normalize_prg_bank(self.bank_registers[6]),
            1 => self.normalize_prg_bank(self.bank_registers[7]),
            2 => self.prg_bank_count_8k - 2,
            _ => self.prg_bank_count_8k - 1,
        }
    }

    fn copy_chr_1k_bank(&self, bank: u8, dst_offset: usize, dst: &mut [u8; CHR_WINDOW_BYTES]) {
        let index = self.normalize_chr_bank(bank) * CHR_BANK_1K;
        let end = index + CHR_BANK_1K;
        dst[dst_offset..dst_offset + CHR_BANK_1K].copy_from_slice(&self.chr_data[index..end]);
    }

    fn copy_chr_2k_bank(&self, bank: u8, dst_offset: usize, dst: &mut [u8; CHR_WINDOW_BYTES]) {
        // Bit 0 is ignored for 2KB banks; the register already stores an even
        // value but mask defensively.
        let even = bank & !1;
        self.copy_chr_1k_bank(even, dst_offset, dst);
        self.copy_chr_1k_bank(even.wrapping_add(1), dst_offset + CHR_BANK_1K, dst);
    }

    /// Returns the currently mapped 8KB CHR window.
    #[must_use]
    pub fn chr_window(&self) -> [u8; CHR_WINDOW_BYTES] {
        let mut mapped = [0_u8; CHR_WINDOW_BYTES];
        self.copy_chr_2k_bank(self.bank_registers[0], 0x0000, &mut mapped);
        self.copy_chr_2k_bank(self.bank_registers[1], 0x0800, &mut mapped);
        self.copy_chr_1k_bank(self.bank_registers[2], 0x1000, &mut mapped);
        self.copy_chr_1k_bank(self.bank_registers[3], 0x1400, &mut mapped);
        self.copy_chr_1k_bank(self.bank_registers[4], 0x1800, &mut mapped);
        self.copy_chr_1k_bank(self.bank_registers[5], 0x1C00, &mut mapped);
        mapped
    }

    /// Returns `true` when mapped CHR should be writable by the PPU (CHR-RAM).
    #[must_use]
    pub fn chr_writable(&self) -> bool {
        self.chr_writable
    }

    /// Synchronizes writable CHR-RAM from the current PPU window.
    ///
    /// Applies the same fixed 2KB/1KB layout in reverse, writing each visible
    /// slot back into its mapped CHR bank.
    pub fn sync_chr_ram_from_ppu_window(&mut self, window: &[u8; CHR_WINDOW_BYTES]) {
        if !self.chr_writable {
            return;
        }
        self.write_chr_2k_bank(self.bank_registers[0], 0x0000, window);
        self.write_chr_2k_bank(self.bank_registers[1], 0x0800, window);
        self.write_chr_1k_bank(self.bank_registers[2], 0x1000, window);
        self.write_chr_1k_bank(self.bank_registers[3], 0x1400, window);
        self.write_chr_1k_bank(self.bank_registers[4], 0x1800, window);
        self.write_chr_1k_bank(self.bank_registers[5], 0x1C00, window);
    }

    fn write_chr_1k_bank(&mut self, bank: u8, src_offset: usize, window: &[u8; CHR_WINDOW_BYTES]) {
        let dst_start = self.normalize_chr_bank(bank) * CHR_BANK_1K;
        let dst_end = dst_start + CHR_BANK_1K;
        self.chr_data[dst_start..dst_end]
            .copy_from_slice(&window[src_offset..src_offset + CHR_BANK_1K]);
    }

    fn write_chr_2k_bank(&mut self, bank: u8, src_offset: usize, window: &[u8; CHR_WINDOW_BYTES]) {
        let even = bank & !1;
        self.write_chr_1k_bank(even, src_offset, window);
        self.write_chr_1k_bank(even.wrapping_add(1), src_offset + CHR_BANK_1K, window);
    }

    /// Reads PRG using the fixed Namco-108 bank mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Writes to the Namco-108 register space.
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }
}

impl Mapper for Namco108 {
    fn read_prg(&self, addr: u16) -> u8 {
        if addr < 0x8000 {
            return 0xFF;
        }
        let slot = ((usize::from(addr) - 0x8000) / PRG_BANK_8K).min(3);
        let bank = self.prg_bank_for_slot(slot);
        let within = (usize::from(addr) - 0x8000) & 0x1FFF;
        self.prg_rom[bank * PRG_BANK_8K + within]
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        if addr < 0x8000 {
            return;
        }
        if addr & 1 == 0 {
            // $8000 (even): bank select. Only bits 0-2 (register index) matter;
            // bits 3-7 are ignored (no PRG mode bit, no CHR inversion).
            self.bank_select = value & 0x07;
        } else {
            // $8001 (odd): bank data into the selected register.
            let register = usize::from(self.bank_select & 0x07);
            let masked = value & 0x3F;
            self.bank_registers[register] = match register {
                0 | 1 => masked & 0x3E,
                _ => masked,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prg_with_bank_markers(banks: usize) -> Vec<u8> {
        let mut prg = vec![0_u8; banks * PRG_BANK_8K];
        for bank in 0..banks {
            prg[bank * PRG_BANK_8K] = bank as u8;
        }
        prg
    }

    fn chr_with_bank_markers(banks: usize) -> Vec<u8> {
        let mut chr = vec![0_u8; banks * CHR_BANK_1K];
        for bank in 0..banks {
            chr[bank * CHR_BANK_1K] = bank as u8;
        }
        chr
    }

    fn select_register(m: &mut Namco108, index: u8, value: u8) {
        m.write_prg(0x8000, index); // even: bank select
        m.write_prg(0x8001, value); // odd: bank data
    }

    #[test]
    fn namco108_prg_banks_switch_via_r6_r7_with_fixed_high() {
        let mut m = Namco108::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
        // R6 -> $8000, R7 -> $A000, $C000 fixed to bank 6, $E000 fixed to bank 7.
        select_register(&mut m, 6, 3);
        select_register(&mut m, 7, 4);
        assert_eq!(m.read_prg(0x8000), 3);
        assert_eq!(m.read_prg(0xA000), 4);
        assert_eq!(m.read_prg(0xC000), 6); // bankcount - 2
        assert_eq!(m.read_prg(0xE000), 7); // bankcount - 1
    }

    #[test]
    fn namco108_2k_chr_banks_ignore_bit0() {
        let mut m = Namco108::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
        // R0 covers $0000-$07FF (2KB). Writing an odd value must drop bit 0.
        select_register(&mut m, 0, 3); // 3 & 0x3E = 2
        let window = m.chr_window();
        assert_eq!(window[0], 2); // 1KB bank 2 at $0000
        assert_eq!(window[CHR_BANK_1K], 3); // next 1KB bank 3 at $0400

        select_register(&mut m, 1, 5); // 5 & 0x3E = 4 -> banks 4,5 at $0800
        let window = m.chr_window();
        assert_eq!(window[0x0800], 4);
        assert_eq!(window[0x0800 + CHR_BANK_1K], 5);
    }

    #[test]
    fn namco108_1k_chr_banks_via_r2_to_r5() {
        let mut m = Namco108::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
        select_register(&mut m, 2, 7);
        select_register(&mut m, 3, 6);
        select_register(&mut m, 4, 5);
        select_register(&mut m, 5, 4);
        let window = m.chr_window();
        assert_eq!(window[0x1000], 7);
        assert_eq!(window[0x1400], 6);
        assert_eq!(window[0x1800], 5);
        assert_eq!(window[0x1C00], 4);
    }

    #[test]
    fn namco108_bank_select_masks_to_index_bits() {
        let mut m = Namco108::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
        // Bits 3-7 of the select value must be ignored: 0xF6 & 0x07 = 6.
        m.write_prg(0x8000, 0xF6);
        m.write_prg(0x8001, 2);
        assert_eq!(m.read_prg(0x8000), 2); // wrote into R6

        m.write_prg(0x8000, 0xFF); // index 7
        m.write_prg(0x8001, 3);
        assert_eq!(m.read_prg(0xA000), 3); // wrote into R7
    }

    #[test]
    fn namco108_register_values_masked_to_six_bits() {
        let mut m = Namco108::from_prg_chr(prg_with_bank_markers(64), chr_with_bank_markers(64));
        // R6 value masked to 6 bits: 0xC5 & 0x3F = 5.
        select_register(&mut m, 6, 0xC5);
        assert_eq!(m.read_prg(0x8000), 5);
    }

    #[test]
    fn namco108_state_round_trips() {
        let mut m = Namco108::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
        select_register(&mut m, 6, 3);
        select_register(&mut m, 2, 5);
        let state = m.state();

        let mut restored =
            Namco108::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
        restored.restore_state(&state);
        assert_eq!(restored.read_prg(0x8000), 3);
        assert_eq!(restored.chr_window()[0x1000], 5);
    }
}
