use super::Mapper;

const PRG_BANK_32K: usize = 32 * 1024;
const CHR_WINDOW_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Mapper 66 (GxROM): switchable 32KB PRG bank and switchable 8KB CHR bank.
pub struct Gxrom {
    prg_bank_count: u8,
    selected_prg_bank: u8,
    chr_bank_count: u8,
    selected_chr_bank: u8,
    prg_rom: Vec<u8>,
    chr_data: Vec<u8>,
    chr_writable: bool,
}

impl Gxrom {
    /// Builds GxROM from raw PRG/CHR data.
    ///
    /// Empty CHR initializes one writable 8KB CHR-RAM window.
    #[must_use]
    pub fn from_prg_chr(prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        let (chr_data, chr_writable) = if chr_rom.is_empty() {
            (vec![0_u8; CHR_WINDOW_BYTES], true)
        } else {
            (chr_rom, false)
        };
        let prg_bank_count = (prg_rom.len() / PRG_BANK_32K) as u8;
        let chr_bank_count = (chr_data.len() / CHR_WINDOW_BYTES) as u8;
        Self {
            prg_bank_count: prg_bank_count.max(1),
            selected_prg_bank: 0,
            chr_bank_count: chr_bank_count.max(1),
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

    /// Returns the currently mapped 8KB CHR window.
    #[must_use]
    pub fn chr_window(&self) -> [u8; CHR_WINDOW_BYTES] {
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        let start = usize::from(self.selected_chr_bank) * CHR_WINDOW_BYTES;
        let end = start + CHR_WINDOW_BYTES;
        window.copy_from_slice(&self.chr_data[start..end]);
        window
    }

    /// Returns `true` when CHR should be writable by the PPU.
    #[must_use]
    pub fn chr_writable(&self) -> bool {
        self.chr_writable
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
        let within_bank = (usize::from(addr) - 0x8000) & 0x7FFF;
        let offset = self.prg_bank_offset(self.selected_prg_bank) + within_bank;
        self.prg_rom[offset]
    }

    fn write_prg(&mut self, _addr: u16, value: u8) {
        let prg_select = (value >> 4) & 0x03;
        let chr_select = value & 0x03;
        self.selected_prg_bank = prg_select % self.prg_bank_count;
        self.selected_chr_bank = chr_select % self.chr_bank_count;
    }
}
