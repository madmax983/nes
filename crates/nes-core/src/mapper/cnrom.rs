use super::Mapper;

const CHR_WINDOW_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Mapper 3 (CNROM): fixed PRG mapping with switchable 8KB CHR banks.
pub struct Cnrom {
    selected_chr_bank: u8,
    chr_bank_count: usize,
    prg_rom: Vec<u8>,
    chr_data: Vec<u8>,
    chr_writable: bool,
}

impl Cnrom {
    /// Builds CNROM from raw PRG/CHR data.
    ///
    /// Empty CHR initializes one writable 8KB CHR-RAM window.
    ///
    /// Inputs are normalized to avoid panics in mapper operations:
    /// - Empty PRG is treated as a single zero byte.
    /// - CHR length is padded to a full 8KB window when needed.
    #[must_use]
    pub fn from_prg_chr(mut prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        if prg_rom.is_empty() {
            prg_rom.push(0);
        }

        let (mut chr_data, chr_writable) = if chr_rom.is_empty() {
            (vec![0_u8; CHR_WINDOW_BYTES], true)
        } else {
            (chr_rom, false)
        };
        let remainder = chr_data.len() % CHR_WINDOW_BYTES;
        if remainder != 0 {
            chr_data.resize(chr_data.len() + (CHR_WINDOW_BYTES - remainder), 0);
        }
        let chr_bank_count = (chr_data.len() / CHR_WINDOW_BYTES).max(1);
        Self {
            selected_chr_bank: 0,
            chr_bank_count,
            prg_rom,
            chr_data,
            chr_writable,
        }
    }

    /// Returns the currently selected CHR bank.
    #[must_use]
    pub fn selected_chr_bank(&self) -> u8 {
        self.selected_chr_bank
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

    /// Reads PRG using fixed CNROM PRG mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Applies PRG-space write as CHR bank select.
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }

    fn prg_offset_for(&self, addr: u16) -> usize {
        if self.prg_rom.is_empty() {
            return 0;
        }
        let base = addr.saturating_sub(0x8000) as usize;
        base % self.prg_rom.len()
    }
}

impl Mapper for Cnrom {
    fn read_prg(&self, addr: u16) -> u8 {
        self.prg_rom[self.prg_offset_for(addr)]
    }

    fn write_prg(&mut self, _addr: u16, value: u8) {
        self.selected_chr_bank = (usize::from(value) % self.chr_bank_count) as u8;
    }
}
