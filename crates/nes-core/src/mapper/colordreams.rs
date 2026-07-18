use super::Mapper;
use serde::{Deserialize, Serialize};

const PRG_BANK_32K: usize = 32 * 1024;
const CHR_WINDOW_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Mapper 11 (Color Dreams): switchable 32KB PRG bank and switchable 8KB CHR bank.
///
/// A single 8-bit register is writable anywhere in `$8000..=$FFFF`:
/// bits 0-1 select the 32KB PRG bank, bits 4-7 select the 8KB CHR bank.
pub struct ColorDreams {
    prg_bank_count: usize,
    selected_prg_bank: u8,
    chr_bank_count: usize,
    selected_chr_bank: u8,
    prg_rom: Vec<u8>,
    chr_data: Vec<u8>,
    chr_writable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ColorDreamsState {
    pub selected_prg_bank: u8,
    pub selected_chr_bank: u8,
}

impl ColorDreams {
    /// Builds Color Dreams from raw PRG/CHR data.
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
    pub(crate) fn state(&self) -> ColorDreamsState {
        ColorDreamsState {
            selected_prg_bank: self.selected_prg_bank,
            selected_chr_bank: self.selected_chr_bank,
        }
    }

    /// Restores state from a snapshot.
    ///
    /// Performance note: Takes state by reference to avoid heap allocations
    /// when restoring structs with large dynamically allocated fields.
    pub(crate) fn restore_state(&mut self, state: &ColorDreamsState) {
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

    /// Returns `true` when CHR should be writable by the PPU (CHR-RAM present).
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

    /// Reads PRG byte through the current 32KB mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Applies a bank-select register write.
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }
}

impl Mapper for ColorDreams {
    fn read_prg(&self, addr: u16) -> u8 {
        let within_bank = usize::from(addr) & 0x7FFF;
        let offset = usize::from(self.selected_prg_bank) * PRG_BANK_32K + within_bank;
        self.prg_rom[offset]
    }

    fn write_prg(&mut self, _addr: u16, value: u8) {
        let prg_select = usize::from(value & 0x03);
        let chr_select = usize::from((value >> 4) & 0x0F);
        self.selected_prg_bank = (prg_select % self.prg_bank_count) as u8;
        self.selected_chr_bank = (chr_select % self.chr_bank_count) as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_dreams_switches_32k_prg_bank() {
        let mut prg = vec![0_u8; 4 * PRG_BANK_32K];
        for bank in 0..4 {
            prg[bank * PRG_BANK_32K] = 0x10 + bank as u8;
        }
        let mut mapper = ColorDreams::from_prg_chr(prg, vec![0_u8; CHR_WINDOW_BYTES]);

        assert_eq!(mapper.read_prg(0x8000), 0x10);
        mapper.write_prg(0x8000, 0x01); // bits 0-1 = PRG bank 1
        assert_eq!(mapper.read_prg(0x8000), 0x11);
        mapper.write_prg(0x9ABC, 0x02); // register writable anywhere in $8000-$FFFF
        assert_eq!(mapper.read_prg(0x8000), 0x12);
        mapper.write_prg(0xFFFF, 0x03); // write anywhere in the window still works
        assert_eq!(mapper.read_prg(0x8000), 0x13);
    }

    #[test]
    fn color_dreams_switches_8k_chr_bank() {
        let mut chr = vec![0_u8; 4 * CHR_WINDOW_BYTES];
        for bank in 0..4 {
            chr[bank * CHR_WINDOW_BYTES] = 0xA0 + bank as u8;
        }
        let mut mapper = ColorDreams::from_prg_chr(vec![0_u8; PRG_BANK_32K], chr);

        assert_eq!(mapper.chr_window()[0], 0xA0);
        mapper.write_prg(0x8000, 0x10); // bits 4-7 = CHR bank 1
        assert_eq!(mapper.chr_window()[0], 0xA1);
        mapper.write_prg(0x8000, 0x30); // CHR bank 3
        assert_eq!(mapper.chr_window()[0], 0xA3);
    }

    #[test]
    fn color_dreams_masks_bank_select_bits() {
        // 2 PRG banks, 2 CHR banks so wrap keeps assertions simple.
        let mut prg = vec![0_u8; 2 * PRG_BANK_32K];
        prg[0] = 0x00;
        prg[PRG_BANK_32K] = 0x01;
        let mut chr = vec![0_u8; 2 * CHR_WINDOW_BYTES];
        chr[0] = 0xB0;
        chr[CHR_WINDOW_BYTES] = 0xB1;
        let mut mapper = ColorDreams::from_prg_chr(prg, chr);

        // Bits 2-3 must not affect PRG select; bits 0-1 only.
        mapper.write_prg(0x8000, 0b0000_1100); // PRG bits 0-1 = 0, CHR bits 4-7 = 0
        assert_eq!(mapper.read_prg(0x8000), 0x00);
        assert_eq!(mapper.chr_window()[0], 0xB0);

        // CHR select uses bits 4-7 only.
        mapper.write_prg(0x8000, 0b0001_0001); // PRG = 1, CHR = 1
        assert_eq!(mapper.read_prg(0x8000), 0x01);
        assert_eq!(mapper.chr_window()[0], 0xB1);
    }

    #[test]
    fn color_dreams_chr_ram_is_writable_when_chr_empty() {
        let mut mapper = ColorDreams::from_prg_chr(vec![0_u8; PRG_BANK_32K], vec![]);
        assert!(mapper.chr_writable());
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        window[0] = 0x77;
        mapper.sync_chr_ram_from_ppu_window(&window);
        assert_eq!(mapper.chr_window()[0], 0x77);
    }

    #[test]
    fn color_dreams_chr_rom_is_not_writable() {
        let mapper =
            ColorDreams::from_prg_chr(vec![0_u8; PRG_BANK_32K], vec![0_u8; CHR_WINDOW_BYTES]);
        assert!(!mapper.chr_writable());
    }

    #[test]
    fn color_dreams_state_round_trips() {
        let mut mapper = ColorDreams::from_prg_chr(
            vec![0_u8; 4 * PRG_BANK_32K],
            vec![0_u8; 4 * CHR_WINDOW_BYTES],
        );
        mapper.write_prg(0x8000, 0x32); // PRG bank 2, CHR bank 3
        let state = mapper.state();
        assert_eq!(state.selected_prg_bank, 2);
        assert_eq!(state.selected_chr_bank, 3);

        let mut restored = ColorDreams::from_prg_chr(
            vec![0_u8; 4 * PRG_BANK_32K],
            vec![0_u8; 4 * CHR_WINDOW_BYTES],
        );
        restored.restore_state(&state);
        assert_eq!(restored.selected_prg_bank(), 2);
        assert_eq!(restored.selected_chr_bank(), 3);
    }
}
