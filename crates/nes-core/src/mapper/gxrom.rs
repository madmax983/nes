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
