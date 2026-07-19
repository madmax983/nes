use super::Mapper;
use serde::{Deserialize, Serialize};

const PRG_BANK_16K: usize = 16 * 1024;
const CHR_WINDOW_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Mapper 71 (Camerica / Codemasters BF9093/BF9097, UxROM-like).
///
/// A 16KB switchable PRG bank is mapped at `$8000..=$BFFF`, selected by the low
/// four bits of writes to `$C000..=$FFFF`. The `$C000..=$FFFF` window is fixed
/// to the last 16KB bank. Writes to `$8000..=$BFFF` are ignored (submapper-0
/// boards hardwire mirroring). CHR is always 8KB CHR-RAM.
pub struct Camerica {
    bank_count: usize,
    selected_bank: usize,
    prg_rom: Vec<u8>,
    chr_ram: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CamericaState {
    pub selected_bank: u8,
}

impl Camerica {
    /// Builds Camerica from raw PRG ROM bytes.
    ///
    /// PRG is zero-padded to at least one 16KB bank and rounded up to a full
    /// bank. CHR is always an 8KB writable CHR-RAM window (these boards have no
    /// CHR ROM).
    #[must_use]
    pub fn from_prg_rom(mut prg_rom: Vec<u8>) -> Self {
        if prg_rom.is_empty() {
            prg_rom.resize(PRG_BANK_16K, 0);
        }
        let remainder = prg_rom.len() % PRG_BANK_16K;
        if remainder != 0 {
            prg_rom.resize(prg_rom.len() + (PRG_BANK_16K - remainder), 0);
        }
        let bank_count = (prg_rom.len() / PRG_BANK_16K).max(1);
        Self {
            bank_count,
            selected_bank: 0,
            prg_rom,
            chr_ram: vec![0_u8; CHR_WINDOW_BYTES],
        }
    }

    /// Returns the currently selected switchable 16KB bank.
    #[must_use]
    pub fn selected_bank(&self) -> u8 {
        self.selected_bank as u8
    }

    #[must_use]
    pub(crate) fn state(&self) -> CamericaState {
        CamericaState {
            selected_bank: self.selected_bank as u8,
        }
    }

        /// Restores state from a snapshot.
    ///
    /// Performance note: Takes state by reference to avoid heap allocations
    /// when restoring structs with large dynamically allocated fields.
    pub(crate) fn restore_state(&mut self, state: &CamericaState) {
        self.selected_bank = usize::from(state.selected_bank) % self.bank_count;
    }

    /// Returns the currently mapped 8KB CHR window (always CHR-RAM).
    #[must_use]
    pub fn chr_window(&self) -> [u8; CHR_WINDOW_BYTES] {
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        window.copy_from_slice(&self.chr_ram);
        window
    }

    /// Returns `true`: Camerica boards always expose 8KB CHR-RAM.
    #[must_use]
    pub fn chr_writable(&self) -> bool {
        true
    }

    /// Synchronizes writable CHR-RAM from the current PPU window.
    pub fn sync_chr_ram_from_ppu_window(&mut self, window: &[u8; CHR_WINDOW_BYTES]) {
        self.chr_ram.copy_from_slice(window);
    }

    /// Reads PRG byte through the current 16KB mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Applies a bank-select register write.
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }

    fn last_bank(&self) -> usize {
        self.bank_count - 1
    }

    fn read_bank(&self, bank: usize, addr: u16) -> u8 {
        let within_bank = usize::from(addr) & 0x3FFF;
        self.prg_rom[bank * PRG_BANK_16K + within_bank]
    }
}

impl Mapper for Camerica {
    fn read_prg(&self, addr: u16) -> u8 {
        if addr < 0xC000 {
            self.read_bank(self.selected_bank, addr)
        } else {
            self.read_bank(self.last_bank(), addr)
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        // Only $C000-$FFFF selects the switchable bank; writes to $8000-$BFFF
        // are ignored on submapper-0 boards.
        if addr >= 0xC000 {
            self.selected_bank = usize::from(value & 0x0F) % self.bank_count;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prg_with_markers(banks: usize) -> Vec<u8> {
        let mut prg = vec![0_u8; banks * PRG_BANK_16K];
        for bank in 0..banks {
            prg[bank * PRG_BANK_16K] = 0x10 + bank as u8;
        }
        prg
    }

    #[test]
    fn camerica_switchable_bank_at_8000() {
        let mut mapper = Camerica::from_prg_rom(prg_with_markers(4));
        assert_eq!(mapper.read_prg(0x8000), 0x10); // bank 0 by default

        mapper.write_prg(0xC000, 0x02); // select bank 2 (via $C000-$FFFF)
        assert_eq!(mapper.read_prg(0x8000), 0x12);

        mapper.write_prg(0xFFFF, 0x01); // select bank 1
        assert_eq!(mapper.read_prg(0x8000), 0x11);
    }

    #[test]
    fn camerica_fixed_last_bank_at_c000() {
        let mut mapper = Camerica::from_prg_rom(prg_with_markers(4));
        // $C000-$FFFF always maps the last bank (index 3 => marker 0x13).
        assert_eq!(mapper.read_prg(0xC000), 0x13);
        mapper.write_prg(0xC000, 0x01); // switch low window, high stays fixed
        assert_eq!(mapper.read_prg(0xC000), 0x13);
        assert_eq!(mapper.read_prg(0x8000), 0x11);
    }

    #[test]
    fn camerica_masks_bank_select_to_low_nibble() {
        let mut mapper = Camerica::from_prg_rom(prg_with_markers(4));
        // Upper nibble bits must be ignored; 0xF2 & 0x0F = 2, then % 4 = 2.
        mapper.write_prg(0xE000, 0xF2);
        assert_eq!(mapper.read_prg(0x8000), 0x12);
    }

    #[test]
    fn camerica_writes_to_8000_bfff_are_noops() {
        let mut mapper = Camerica::from_prg_rom(prg_with_markers(4));
        mapper.write_prg(0xC000, 0x02); // bank 2
        assert_eq!(mapper.read_prg(0x8000), 0x12);
        mapper.write_prg(0x8000, 0x03); // ignored
        mapper.write_prg(0xBFFF, 0x01); // ignored
        assert_eq!(mapper.read_prg(0x8000), 0x12);
    }

    #[test]
    fn camerica_chr_ram_is_writable() {
        let mut mapper = Camerica::from_prg_rom(prg_with_markers(2));
        assert!(mapper.chr_writable());
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        window[0] = 0x9A;
        window[CHR_WINDOW_BYTES - 1] = 0xBC;
        mapper.sync_chr_ram_from_ppu_window(&window);
        let read = mapper.chr_window();
        assert_eq!(read[0], 0x9A);
        assert_eq!(read[CHR_WINDOW_BYTES - 1], 0xBC);
    }

    #[test]
    fn camerica_state_round_trips() {
        let mut mapper = Camerica::from_prg_rom(prg_with_markers(4));
        mapper.write_prg(0xC000, 0x03);
        let state = mapper.state();
        assert_eq!(state.selected_bank, 3);

        let mut restored = Camerica::from_prg_rom(prg_with_markers(4));
        restored.restore_state(&state);
        assert_eq!(restored.selected_bank(), 3);
        assert_eq!(restored.read_prg(0x8000), 0x13);
    }
}
