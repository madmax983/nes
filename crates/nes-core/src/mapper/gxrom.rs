use alloc::{vec, vec::Vec};

use super::Mapper;
use serde::{Deserialize, Serialize};

const PRG_BANK_32K: usize = 32 * 1024;
const CHR_WINDOW_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Mapper 66 (GxROM): switchable 32KB PRG bank and switchable 8KB CHR bank.
pub struct Gxrom {
    prg_bank_count: usize,
    selected_prg_bank: u8,
    chr_bank_count: usize,
    selected_chr_bank: u8,
    prg_rom: Vec<u8>,
    chr_data: Vec<u8>,
    chr_writable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GxromState {
    pub selected_prg_bank: u8,
    pub selected_chr_bank: u8,
}

impl Gxrom {
    /// Builds GxROM from raw PRG/CHR data.
    ///
    /// Empty CHR initializes one writable 8KB CHR-RAM window.
    ///
    /// Inputs are normalized to avoid panics in mapper operations:
    /// - PRG is zero-padded to at least one 32KB bank and rounded up to a full bank.
    /// - Non-empty CHR is rounded up to a full 8KB window.
    #[must_use]
    pub fn from_prg_chr(mut prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        if prg_rom.len() < PRG_BANK_32K {
            prg_rom.resize(PRG_BANK_32K, 0);
        }
        let prg_remainder = prg_rom.len() % PRG_BANK_32K;
        if prg_remainder != 0 {
            prg_rom.resize(prg_rom.len() + (PRG_BANK_32K - prg_remainder), 0);
        }

        let (mut chr_data, chr_writable) = if chr_rom.is_empty() {
            (vec![0_u8; CHR_WINDOW_BYTES], true)
        } else {
            (chr_rom, false)
        };
        let chr_remainder = chr_data.len() % CHR_WINDOW_BYTES;
        if chr_remainder != 0 {
            chr_data.resize(chr_data.len() + (CHR_WINDOW_BYTES - chr_remainder), 0);
        }

        let prg_bank_count = (prg_rom.len() / PRG_BANK_32K).max(1);
        let chr_bank_count = (chr_data.len() / CHR_WINDOW_BYTES).max(1);

        Self {
            prg_bank_count,
            selected_prg_bank: 0,
            chr_bank_count,
            selected_chr_bank: 0,
            prg_rom,
            chr_data,
            chr_writable,
        }
    }

    /// Returns the currently selected 32KB PRG bank.
    #[must_use]
    pub fn selected_prg_bank(&self) -> u8 {
        self.selected_prg_bank
    }

    /// Returns the currently selected 8KB CHR bank.
    #[must_use]
    pub fn selected_chr_bank(&self) -> u8 {
        self.selected_chr_bank
    }

    #[must_use]
    pub(crate) fn state(&self) -> GxromState {
        GxromState {
            selected_prg_bank: self.selected_prg_bank,
            selected_chr_bank: self.selected_chr_bank,
        }
    }

    pub(crate) fn restore_state(&mut self, state: GxromState) {
        self.selected_prg_bank = (usize::from(state.selected_prg_bank) % self.prg_bank_count) as u8;
        self.selected_chr_bank = (usize::from(state.selected_chr_bank) % self.chr_bank_count) as u8;
    }

    /// Returns the currently mapped 8KB CHR window.
    #[must_use]
    pub fn chr_window(&self) -> [u8; CHR_WINDOW_BYTES] {
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        let start = usize::from(self.selected_chr_bank) * CHR_WINDOW_BYTES;
        let end = start + CHR_WINDOW_BYTES;
        if let Some(mapped_window) = self.chr_data.get(start..end) {
            window.copy_from_slice(mapped_window);
        }
        window
    }

    /// Returns `true` when CHR should be writable by the PPU.
    #[must_use]
    pub fn chr_writable(&self) -> bool {
        self.chr_writable
    }

    /// Synchronizes writable CHR-RAM from the current PPU window.
    pub fn sync_chr_ram_from_ppu_window(&mut self, window: &[u8; CHR_WINDOW_BYTES]) {
        if !self.chr_writable {
            return;
        }

        let start = usize::from(self.selected_chr_bank) * CHR_WINDOW_BYTES;
        let end = start + CHR_WINDOW_BYTES;
        self.chr_data[start..end].copy_from_slice(window);
    }

    /// Reads PRG byte through current GxROM 32KB mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Applies GxROM bank register writes.
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }

    fn prg_bank_offset(&self, bank: u8) -> usize {
        usize::from(bank) * PRG_BANK_32K
    }
}

impl Mapper for Gxrom {
    fn read_prg(&self, addr: u16) -> u8 {
        let within_bank = usize::from(addr) & 0x7FFF;
        let offset = self.prg_bank_offset(self.selected_prg_bank) + within_bank;
        self.prg_rom[offset]
    }

    fn write_prg(&mut self, _addr: u16, value: u8) {
        let prg_select = usize::from((value >> 4) & 0x03);
        let chr_select = usize::from(value & 0x03);
        self.selected_prg_bank = (prg_select % self.prg_bank_count) as u8;
        self.selected_chr_bank = (chr_select % self.chr_bank_count) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gxrom_from_prg_chr_more_math_operators() {
        // Less than 32k
        let mapper = Gxrom::from_prg_chr(vec![0_u8; 32 * 1024 - 1], vec![]);
        assert_eq!(mapper.prg_bank_count, 1);
        assert_eq!(mapper.prg_rom.len(), 32 * 1024);

        // Slightly over 32k.
        let mapper3 = Gxrom::from_prg_chr(vec![0_u8; 32 * 1024 + 1], vec![]);
        assert_eq!(mapper3.prg_bank_count, 2);
        assert_eq!(mapper3.prg_rom.len(), 64 * 1024);

        // CHR slightly over 8k.
        let mapper4 = Gxrom::from_prg_chr(vec![], vec![0_u8; 8 * 1024 + 1]);
        assert_eq!(mapper4.chr_bank_count, 2);
        assert_eq!(mapper4.chr_data.len(), 16 * 1024);

        // Ensure less than 8k logic works.
        let mapper5 = Gxrom::from_prg_chr(vec![], vec![0_u8; 8 * 1024 - 1]);
        assert_eq!(mapper5.chr_bank_count, 1);
        assert_eq!(mapper5.chr_data.len(), 8 * 1024);

        // Exactly 8k
        let mapper6 = Gxrom::from_prg_chr(vec![], vec![0_u8; 8 * 1024]);
        assert_eq!(mapper6.chr_bank_count, 1);
        assert_eq!(mapper6.chr_data.len(), 8 * 1024);
    }

    #[test]
    fn gxrom_from_prg_chr_more_math_operators3() {
        // Less than 32k
        let mapper = Gxrom::from_prg_chr(vec![0_u8; 32 * 1024 - 1], vec![]);
        assert_eq!(mapper.prg_bank_count, 1);
        assert_eq!(mapper.prg_rom.len(), 32 * 1024);

        // Exact 32k.
        let mapper2 = Gxrom::from_prg_chr(vec![0_u8; 32 * 1024], vec![]);
        assert_eq!(mapper2.prg_bank_count, 1);
        assert_eq!(mapper2.prg_rom.len(), 32 * 1024);

        // Ensure less than PRG_BANK_32K logic `<=` works.
        // It requires an EXACT 32k input, but if `if prg_rom.len() <= PRG_BANK_32K`
        // was evaluated, it would unnecessarily pad it or do operations resulting in a different state
        // under Cargo Mutants logic if `resize` works correctly? Wait, resize doesn't do anything if
        // the target length is the same. Wait, if it's `<`, resize is skipped for `== 32 * 1024`.
        // If it's `<=`, it's executed, but `resize(32768)` on a vec of len `32768` is a no-op!
        // This is an **equivalent mutant**.
    }

    #[test]
    fn gxrom_from_prg_chr_more_math_operators2() {
        // Less than 32k
        let mapper = Gxrom::from_prg_chr(vec![0_u8; 32 * 1024 - 1], vec![]);
        assert_eq!(mapper.prg_bank_count, 1);
        assert_eq!(mapper.prg_rom.len(), 32 * 1024);

        // Exact 32k.
        let mapper2 = Gxrom::from_prg_chr(vec![0_u8; 32 * 1024], vec![]);
        assert_eq!(mapper2.prg_bank_count, 1);
        assert_eq!(mapper2.prg_rom.len(), 32 * 1024);

        // Slightly over 32k.
        let mapper3 = Gxrom::from_prg_chr(vec![0_u8; 32 * 1024 + 1], vec![]);
        assert_eq!(mapper3.prg_bank_count, 2);
        assert_eq!(mapper3.prg_rom.len(), 64 * 1024);

        // CHR slightly over 8k.
        let mapper4 = Gxrom::from_prg_chr(vec![], vec![0_u8; 8 * 1024 + 1]);
        assert_eq!(mapper4.chr_bank_count, 2);
        assert_eq!(mapper4.chr_data.len(), 16 * 1024);

        // Ensure less than 8k logic works.
        let mapper5 = Gxrom::from_prg_chr(vec![], vec![0_u8; 8 * 1024 - 1]);
        assert_eq!(mapper5.chr_bank_count, 1);
        assert_eq!(mapper5.chr_data.len(), 8 * 1024);

        // Exactly 8k
        let mapper6 = Gxrom::from_prg_chr(vec![], vec![0_u8; 8 * 1024]);
        assert_eq!(mapper6.chr_bank_count, 1);
        assert_eq!(mapper6.chr_data.len(), 8 * 1024);
    }

    #[test]
    fn gxrom_read_and_write_prg() {
        let mut prg_rom = vec![0; 64 * 1024];
        prg_rom[0x0000] = 0xAA; // Bank 0, offset 0
        prg_rom[32 * 1024] = 0xBB; // Bank 1, offset 0

        let mut chr_rom = vec![0; 16 * 1024];
        chr_rom[0x0000] = 0xCC; // Bank 0, offset 0
        chr_rom[8 * 1024] = 0xDD; // Bank 1, offset 0

        let mut mapper = Gxrom::from_prg_chr(prg_rom, chr_rom);

        // Initially Bank 0
        assert_eq!(mapper.read_prg(0x8000), 0xAA);
        assert_eq!(mapper.chr_window()[0], 0xCC);

        // Write to switch to PRG Bank 1 and CHR Bank 1
        // value format is prg_select << 4 | chr_select
        mapper.write_prg(0x8000, 0x11);

        assert_eq!(mapper.read_prg(0x8000), 0xBB);
        assert_eq!(mapper.chr_window()[0], 0xDD);
    }

    #[test]
    fn should_sync_chr_ram_when_writable() {
        // Create a GxROM mapper with empty CHR ROM, making it writable (CHR-RAM)
        let mut mapper = Gxrom::from_prg_chr(vec![0; 32 * 1024], vec![]);
        assert!(mapper.chr_writable());

        // Update the CHR window
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        window[0] = 42;
        window[CHR_WINDOW_BYTES - 1] = 84;

        mapper.sync_chr_ram_from_ppu_window(&window);

        // Verify the mapper's internal CHR RAM was updated
        let updated_window = mapper.chr_window();
        assert_eq!(updated_window[0], 42);
        assert_eq!(updated_window[CHR_WINDOW_BYTES - 1], 84);
    }

    #[test]
    fn should_not_sync_chr_ram_when_not_writable() {
        // Create a GxROM mapper with non-empty CHR ROM, making it read-only
        let original_chr = vec![0; CHR_WINDOW_BYTES];
        let mut mapper = Gxrom::from_prg_chr(vec![0; 32 * 1024], original_chr);
        assert!(!mapper.chr_writable());

        // Attempt to update the CHR window
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        window[0] = 42;

        mapper.sync_chr_ram_from_ppu_window(&window);

        // Verify the mapper's internal CHR ROM was NOT updated
        let unchanged_window = mapper.chr_window();
        assert_eq!(unchanged_window[0], 0);
    }

    #[test]
    fn should_restore_state() {
        let mut mapper = Gxrom::from_prg_chr(vec![0; 64 * 1024], vec![0; 16 * 1024]);

        // Change banks
        mapper.write_prg(0x8000, 0x11); // PRG bank 1, CHR bank 1

        let state = mapper.state();
        assert_eq!(state.selected_prg_bank, 1);
        assert_eq!(state.selected_chr_bank, 1);

        // Reset banks
        mapper.write_prg(0x8000, 0x00);

        // Restore
        mapper.restore_state(state);
        assert_eq!(mapper.selected_prg_bank(), 1);
        assert_eq!(mapper.selected_chr_bank(), 1);
    }

    #[test]
    fn gxrom_from_prg_chr_pads_empty_prg() {
        let mapper = Gxrom::from_prg_chr(vec![], vec![]);
        assert_eq!(mapper.prg_bank_count, 1);
        assert_eq!(mapper.prg_rom.len(), PRG_BANK_32K);
    }

    #[test]
    fn gxrom_from_prg_chr_pads_short_prg() {
        // Test `if prg_rom.len() < PRG_BANK_32K` branch
        let prg_rom = vec![0; PRG_BANK_32K - 1];
        let mapper = Gxrom::from_prg_chr(prg_rom, vec![]);

        // Should pad to 1 bank
        assert_eq!(mapper.prg_bank_count, 1);
        assert_eq!(mapper.prg_rom.len(), PRG_BANK_32K);
    }

    #[test]
    fn gxrom_from_prg_chr_pads_chr() {
        // Test CHR padding logic: `chr_data.resize(chr_data.len() + (CHR_WINDOW_BYTES - chr_remainder), 0);`
        let chr_rom = vec![0; CHR_WINDOW_BYTES + 1];
        let mapper = Gxrom::from_prg_chr(vec![], chr_rom);

        // Should pad CHR to 2 banks
        assert_eq!(mapper.chr_bank_count, 2);
        assert_eq!(mapper.chr_data.len(), 2 * CHR_WINDOW_BYTES);
    }

    #[test]
    fn gxrom_from_prg_chr_pads_partial_bank() {
        // Test `prg_rom.resize(prg_rom.len() + (PRG_BANK_32K - prg_remainder), 0);` and `%`
        // 32KB + 1 byte remainder -> should pad to 64KB
        let prg_rom = vec![0; PRG_BANK_32K + 1];
        let mapper = Gxrom::from_prg_chr(prg_rom, vec![]);

        // Should pad to 2 banks
        assert_eq!(mapper.prg_bank_count, 2);
        assert_eq!(mapper.prg_rom.len(), 2 * PRG_BANK_32K);
    }

    #[test]
    fn gxrom_from_prg_chr_exact_32k() {
        let prg_rom = vec![0; PRG_BANK_32K];
        let mapper = Gxrom::from_prg_chr(prg_rom, vec![]);
        assert_eq!(mapper.prg_bank_count, 1);
        assert_eq!(mapper.prg_rom.len(), PRG_BANK_32K);
    }
}
