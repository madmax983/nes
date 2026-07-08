use crate::rom::NametableMirroring;
use serde::{Deserialize, Serialize};

use super::Mapper;

const PRG_BANK_16K: usize = 16 * 1024;
const CHR_BANK_4K: usize = 4 * 1024;
const CHR_WINDOW_BYTES: usize = 8 * 1024;
/// 8KB of on-cartridge PRG-RAM (WRAM), mapped at CPU `$6000..=$7FFF`.
const PRG_RAM_BYTES: usize = 8 * 1024;
const PRG_RAM_BASE: u16 = 0x6000;
const PRG_RAM_END: u16 = 0x7FFF;

/// MMC4 needs at least two 16KB PRG banks: one switchable at `$8000..=$BFFF`
/// plus the fixed last bank at `$C000..=$FFFF`.
const MIN_PRG_BANKS_16K: usize = 2;

/// The two possible states of an MMC2/MMC4 CHR latch.
///
/// Each latch is flipped by the PPU fetching a specific pattern-table tile
/// (`$FD` or `$FE`) and selects which of a register pair supplies the mapped
/// 4KB CHR bank. Real hardware powers up with both latches reading `$FE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ChrLatch {
    /// Latched by a fetch of tile `$FD` (`$_FD8`); selects the `$B000`/`$D000` bank.
    Fd,
    /// Latched by a fetch of tile `$FE` (`$_FE8`); selects the `$C000`/`$E000` bank.
    Fe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Mapper 10 (MMC4, "Fire Emblem"): switchable 16KB PRG at `$8000`, a fixed
/// 16KB bank at `$C000`, 8KB PRG-RAM, and two independently latched 4KB CHR
/// halves.
///
/// MMC4 shares MMC2's CHR A12/tile latch (see [`Mmc4::notify_ppu_chr_fetch`])
/// and the same register addresses; only the PRG banking and the presence of
/// PRG-RAM differ.
///
/// Register writes (address decoded by high nibble):
/// * `$A000..=$AFFF` — PRG bank select (bits 0-3) for `$8000..=$BFFF` (16KB).
/// * `$B000..=$BFFF` — 4KB CHR bank for `$0000` used when latch0 == `$FD`.
/// * `$C000..=$CFFF` — 4KB CHR bank for `$0000` used when latch0 == `$FE`.
/// * `$D000..=$DFFF` — 4KB CHR bank for `$1000` used when latch1 == `$FD`.
/// * `$E000..=$EFFF` — 4KB CHR bank for `$1000` used when latch1 == `$FE`.
/// * `$F000..=$FFFF` — mirroring (bit 0: 0 = Vertical, 1 = Horizontal).
pub struct Mmc4 {
    prg_bank_count_16k: usize,
    prg_rom: Vec<u8>,
    chr_bank_count_4k: usize,
    chr_data: Vec<u8>,
    chr_writable: bool,
    /// 16KB PRG bank mapped at `$8000..=$BFFF` (reg `$A000`).
    prg_bank: u8,
    /// 4KB CHR bank for `$0000` selected when latch0 == `$FD` (reg `$B000`).
    chr_bank_fd0: u8,
    /// 4KB CHR bank for `$0000` selected when latch0 == `$FE` (reg `$C000`).
    chr_bank_fe0: u8,
    /// 4KB CHR bank for `$1000` selected when latch1 == `$FD` (reg `$D000`).
    chr_bank_fd1: u8,
    /// 4KB CHR bank for `$1000` selected when latch1 == `$FE` (reg `$E000`).
    chr_bank_fe1: u8,
    /// Latch for the low ($0000) pattern-table half.
    latch0: ChrLatch,
    /// Latch for the high ($1000) pattern-table half.
    latch1: ChrLatch,
    mirroring: NametableMirroring,
    /// 8KB PRG-RAM mapped at `$6000..=$7FFF` (always enabled on MMC4).
    prg_ram: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Mmc4State {
    pub prg_bank: u8,
    pub chr_bank_fd0: u8,
    pub chr_bank_fe0: u8,
    pub chr_bank_fd1: u8,
    pub chr_bank_fe1: u8,
    pub latch0: ChrLatch,
    pub latch1: ChrLatch,
    pub mirroring: NametableMirroring,
    pub prg_ram: Vec<u8>,
}

impl Mmc4 {
    /// Builds MMC4 from raw PRG/CHR data.
    ///
    /// Inputs are normalized so mapper operations never panic:
    /// - PRG is zero-padded to at least two 16KB banks and rounded up to a full bank.
    /// - Empty CHR becomes one writable 8KB CHR-RAM window (defensive; real MMC4
    ///   carts ship CHR-ROM). Non-empty CHR is padded up to a full 4KB bank.
    #[must_use]
    pub fn from_prg_chr(mut prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        let min_prg_bytes = MIN_PRG_BANKS_16K * PRG_BANK_16K;
        if prg_rom.len() < min_prg_bytes {
            prg_rom.resize(min_prg_bytes, 0);
        }
        let prg_remainder = prg_rom.len() % PRG_BANK_16K;
        if prg_remainder != 0 {
            prg_rom.resize(prg_rom.len() + (PRG_BANK_16K - prg_remainder), 0);
        }
        let prg_bank_count_16k = prg_rom.len() / PRG_BANK_16K;

        let (mut chr_data, chr_writable) = if chr_rom.is_empty() {
            (vec![0_u8; CHR_WINDOW_BYTES], true)
        } else {
            (chr_rom, false)
        };
        if chr_data.len() < CHR_WINDOW_BYTES {
            chr_data.resize(CHR_WINDOW_BYTES, 0);
        }
        let chr_remainder = chr_data.len() % CHR_BANK_4K;
        if chr_remainder != 0 {
            chr_data.resize(chr_data.len() + (CHR_BANK_4K - chr_remainder), 0);
        }
        let chr_bank_count_4k = chr_data.len() / CHR_BANK_4K;

        Self {
            prg_bank_count_16k,
            prg_rom,
            chr_bank_count_4k,
            chr_data,
            chr_writable,
            prg_bank: 0,
            chr_bank_fd0: 0,
            chr_bank_fe0: 0,
            chr_bank_fd1: 0,
            chr_bank_fe1: 0,
            latch0: ChrLatch::Fe,
            latch1: ChrLatch::Fe,
            mirroring: NametableMirroring::Vertical,
            prg_ram: vec![0_u8; PRG_RAM_BYTES],
        }
    }

    #[must_use]
    pub(crate) fn state(&self) -> Mmc4State {
        Mmc4State {
            prg_bank: self.prg_bank,
            chr_bank_fd0: self.chr_bank_fd0,
            chr_bank_fe0: self.chr_bank_fe0,
            chr_bank_fd1: self.chr_bank_fd1,
            chr_bank_fe1: self.chr_bank_fe1,
            latch0: self.latch0,
            latch1: self.latch1,
            mirroring: self.mirroring,
            prg_ram: self.prg_ram.clone(),
        }
    }

    pub(crate) fn restore_state(&mut self, state: Mmc4State) {
        self.prg_bank = state.prg_bank;
        self.chr_bank_fd0 = state.chr_bank_fd0;
        self.chr_bank_fe0 = state.chr_bank_fe0;
        self.chr_bank_fd1 = state.chr_bank_fd1;
        self.chr_bank_fe1 = state.chr_bank_fe1;
        self.latch0 = state.latch0;
        self.latch1 = state.latch1;
        self.mirroring = state.mirroring;
        if state.prg_ram.len() == self.prg_ram.len() {
            self.prg_ram.copy_from_slice(&state.prg_ram);
        } else {
            self.prg_ram = state.prg_ram;
            self.prg_ram.resize(PRG_RAM_BYTES, 0);
        }
    }

    #[must_use]
    fn normalize_prg_bank(&self, raw: u8) -> usize {
        usize::from(raw) % self.prg_bank_count_16k.max(1)
    }

    #[must_use]
    fn normalize_chr_bank(&self, raw: u8) -> usize {
        usize::from(raw) % self.chr_bank_count_4k.max(1)
    }

    /// The 4KB CHR bank currently selected for the low ($0000) half.
    #[must_use]
    fn low_half_bank(&self) -> u8 {
        match self.latch0 {
            ChrLatch::Fd => self.chr_bank_fd0,
            ChrLatch::Fe => self.chr_bank_fe0,
        }
    }

    /// The 4KB CHR bank currently selected for the high ($1000) half.
    #[must_use]
    fn high_half_bank(&self) -> u8 {
        match self.latch1 {
            ChrLatch::Fd => self.chr_bank_fd1,
            ChrLatch::Fe => self.chr_bank_fe1,
        }
    }

    fn copy_chr_4k_bank(&self, bank: u8, dst_offset: usize, dst: &mut [u8; CHR_WINDOW_BYTES]) {
        let src = self.normalize_chr_bank(bank) * CHR_BANK_4K;
        dst[dst_offset..dst_offset + CHR_BANK_4K]
            .copy_from_slice(&self.chr_data[src..src + CHR_BANK_4K]);
    }

    /// Returns the currently mapped 8KB CHR window: two 4KB halves, each chosen
    /// by its own latch.
    #[must_use]
    pub fn chr_window(&self) -> [u8; CHR_WINDOW_BYTES] {
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        self.copy_chr_4k_bank(self.low_half_bank(), 0x0000, &mut window);
        self.copy_chr_4k_bank(self.high_half_bank(), 0x1000, &mut window);
        window
    }

    /// Returns `true` when mapped CHR should be writable by the PPU (CHR-RAM).
    #[must_use]
    pub fn chr_writable(&self) -> bool {
        self.chr_writable
    }

    /// Synchronizes writable CHR-RAM from the current PPU window, writing each
    /// visible 4KB half back into its mapped bank.
    pub fn sync_chr_ram_from_ppu_window(&mut self, window: &[u8; CHR_WINDOW_BYTES]) {
        if !self.chr_writable {
            return;
        }
        let low = self.normalize_chr_bank(self.low_half_bank()) * CHR_BANK_4K;
        self.chr_data[low..low + CHR_BANK_4K].copy_from_slice(&window[..CHR_BANK_4K]);
        let high = self.normalize_chr_bank(self.high_half_bank()) * CHR_BANK_4K;
        self.chr_data[high..high + CHR_BANK_4K]
            .copy_from_slice(&window[0x1000..0x1000 + CHR_BANK_4K]);
    }

    /// Returns the current nametable mirroring mode (reg `$F000`).
    #[must_use]
    pub fn mirroring(&self) -> NametableMirroring {
        self.mirroring
    }

    /// Notifies the mapper of a PPU pattern-table fetch at `addr` (a 14-bit CHR
    /// address). Flips the CHR latches on the trigger tiles and returns `true`
    /// when a latch value actually changed. Identical to MMC2's latch.
    #[must_use]
    pub fn notify_ppu_chr_fetch(&mut self, addr: u16) -> bool {
        let before = (self.latch0, self.latch1);
        match addr & 0x1FF8 {
            0x0FD8 => self.latch0 = ChrLatch::Fd,
            0x0FE8 => self.latch0 = ChrLatch::Fe,
            0x1FD8 => self.latch1 = ChrLatch::Fd,
            0x1FE8 => self.latch1 = ChrLatch::Fe,
            _ => {}
        }
        before != (self.latch0, self.latch1)
    }

    /// Reads a byte from the `$6000..=$7FFF` PRG-RAM window (always enabled).
    /// Shared by the `Mapper::read_prg` `$6000` arm and the CPU-bus
    /// `read_prg_ram` route so the two can never diverge.
    #[must_use]
    fn prg_ram_read(&self, addr: u16) -> u8 {
        self.prg_ram[usize::from(addr - PRG_RAM_BASE)]
    }

    /// Writes a byte into the `$6000..=$7FFF` PRG-RAM window.
    fn prg_ram_write(&mut self, addr: u16, value: u8) {
        self.prg_ram[usize::from(addr - PRG_RAM_BASE)] = value;
    }

    /// Reads PRG-RAM for the CPU bus, or `None` when `addr` is outside the
    /// `$6000..=$7FFF` window.
    #[must_use]
    pub fn read_prg_ram(&self, addr: u16) -> Option<u8> {
        (PRG_RAM_BASE..=PRG_RAM_END)
            .contains(&addr)
            .then(|| self.prg_ram_read(addr))
    }

    /// Writes PRG-RAM from the CPU bus; ignores addresses outside
    /// `$6000..=$7FFF`.
    pub fn write_prg_ram(&mut self, addr: u16, value: u8) {
        if (PRG_RAM_BASE..=PRG_RAM_END).contains(&addr) {
            self.prg_ram_write(addr, value);
        }
    }

    /// Reads PRG using the MMC4 bank mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Writes to the MMC4 register space.
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }
}

impl Mapper for Mmc4 {
    fn read_prg(&self, addr: u16) -> u8 {
        if (PRG_RAM_BASE..=PRG_RAM_END).contains(&addr) {
            return self.prg_ram_read(addr);
        }
        if addr < 0x8000 {
            return 0xFF;
        }
        let bank = if addr < 0xC000 {
            self.normalize_prg_bank(self.prg_bank) // $8000-$BFFF switchable 16KB
        } else {
            self.prg_bank_count_16k - 1 // $C000-$FFFF fixed last 16KB
        };
        let within = (usize::from(addr) - 0x8000) & 0x3FFF;
        self.prg_rom[bank * PRG_BANK_16K + within]
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        if (PRG_RAM_BASE..=PRG_RAM_END).contains(&addr) {
            self.prg_ram_write(addr, value);
            return;
        }
        if addr < 0x8000 {
            return;
        }
        match addr & 0xF000 {
            0xA000 => self.prg_bank = value & 0x0F,
            0xB000 => self.chr_bank_fd0 = value & 0x1F,
            0xC000 => self.chr_bank_fe0 = value & 0x1F,
            0xD000 => self.chr_bank_fd1 = value & 0x1F,
            0xE000 => self.chr_bank_fe1 = value & 0x1F,
            0xF000 => {
                self.mirroring = if value & 1 == 0 {
                    NametableMirroring::Vertical
                } else {
                    NametableMirroring::Horizontal
                };
            }
            // $8000-$9FFF are unmapped on MMC4.
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prg_with_bank_markers(banks_16k: usize) -> Vec<u8> {
        let mut prg = vec![0_u8; banks_16k * PRG_BANK_16K];
        for bank in 0..banks_16k {
            prg[bank * PRG_BANK_16K] = bank as u8;
        }
        prg
    }

    fn chr_with_bank_markers(banks_4k: usize) -> Vec<u8> {
        let mut chr = vec![0_u8; banks_4k * CHR_BANK_4K];
        for bank in 0..banks_4k {
            chr[bank * CHR_BANK_4K] = bank as u8;
        }
        chr
    }

    #[test]
    fn mmc4_prg_switchable_16k_and_fixed_last() {
        let mut m = Mmc4::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(4));
        assert_eq!(m.read_prg(0x8000), 0); // switchable defaults to bank 0
        m.write_prg(0xA000, 2); // $8000-$BFFF -> bank 2
        assert_eq!(m.read_prg(0x8000), 2);
        assert_eq!(m.read_prg(0xC000), 3); // fixed last (4 - 1)
    }

    #[test]
    fn mmc4_prg_ram_round_trip() {
        let mut m = Mmc4::from_prg_chr(prg_with_bank_markers(2), chr_with_bank_markers(4));
        m.write_prg(0x6000, 0xAB);
        m.write_prg(0x7FFF, 0xCD);
        assert_eq!(m.read_prg(0x6000), 0xAB);
        assert_eq!(m.read_prg(0x7FFF), 0xCD);
        assert_eq!(m.read_prg_ram(0x6000), Some(0xAB));
        m.write_prg_ram(0x6001, 0x42);
        assert_eq!(m.read_prg(0x6001), 0x42);
    }

    #[test]
    fn mmc4_chr_latch_matches_mmc2_semantics() {
        let mut m = Mmc4::from_prg_chr(prg_with_bank_markers(2), chr_with_bank_markers(8));
        m.write_prg(0xB000, 1); // FD low
        m.write_prg(0xC000, 4); // FE low
        m.write_prg(0xD000, 2); // FD high
        m.write_prg(0xE000, 7); // FE high

        // Default latches == FE.
        assert_eq!(m.chr_window()[0x0000], 4);
        assert_eq!(m.chr_window()[0x1000], 7);

        assert!(!m.notify_ppu_chr_fetch(0x0000)); // non-trigger
        assert!(m.notify_ppu_chr_fetch(0x0FD8)); // latch0 -> FD
        assert_eq!(m.chr_window()[0x0000], 1);
        assert!(m.notify_ppu_chr_fetch(0x0FE8)); // latch0 -> FE
        assert_eq!(m.chr_window()[0x0000], 4);

        assert!(m.notify_ppu_chr_fetch(0x1FD8)); // latch1 -> FD
        assert_eq!(m.chr_window()[0x1000], 2);
        assert!(m.notify_ppu_chr_fetch(0x1FE8)); // latch1 -> FE
        assert_eq!(m.chr_window()[0x1000], 7);
        assert!(!m.notify_ppu_chr_fetch(0x1FE8)); // idempotent
    }

    #[test]
    fn mmc4_mirroring_register_toggles() {
        let mut m = Mmc4::from_prg_chr(prg_with_bank_markers(2), chr_with_bank_markers(4));
        m.write_prg(0xF000, 0x00);
        assert_eq!(m.mirroring(), NametableMirroring::Vertical);
        m.write_prg(0xF000, 0x01);
        assert_eq!(m.mirroring(), NametableMirroring::Horizontal);
    }

    #[test]
    fn mmc4_state_round_trips() {
        let mut m = Mmc4::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
        m.write_prg(0xA000, 2);
        m.write_prg(0xB000, 1);
        m.write_prg(0xE000, 5);
        m.write_prg(0xF000, 0x01);
        m.write_prg(0x6000, 0x9A);
        let _ = m.notify_ppu_chr_fetch(0x0FD8);

        let state = m.state();
        assert_eq!(state.prg_ram[0], 0x9A);

        let mut restored = Mmc4::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
        restored.restore_state(state);
        assert_eq!(restored.read_prg(0x8000), 2);
        assert_eq!(restored.mirroring(), NametableMirroring::Horizontal);
        assert_eq!(restored.read_prg(0x6000), 0x9A);
        assert_eq!(restored.chr_window()[0x0000], 1); // FD low bank after latch
    }
}
