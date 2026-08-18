//! Sunsoft FME-7 (iNES Mapper 69)
//!
//! The FME-7 is a powerful mapper developed by Sunsoft, supporting up to 512KB of PRG-ROM
//! and 256KB of CHR-ROM. It features fine-grained 1KB CHR banking and a CPU-cycle-based
//! IRQ timer.
//!
//! **Why it exists:** Sunsoft needed a mapper that could handle large, complex games
//! like *Batman: Return of the Joker* and *Gimmick!*. The FME-7 provided the necessary
//! flexibility with its numerous banking registers and precise IRQ timing, allowing for
//! advanced visual effects and parallax scrolling.
//!
//! ## Examples
//!
//! ```
//! use nes_core::mapper::Fme7;
//!
//! // Example PRG and CHR ROM data (minimum required sizes)
//! let prg_rom = vec![0; 16 * 1024]; // 16KB PRG
//! let chr_rom = vec![0; 8 * 1024];  // 8KB CHR
//!
//! let mapper = Fme7::from_prg_chr(prg_rom, chr_rom);
//! ```

use crate::rom::NametableMirroring;
use serde::{Deserialize, Serialize};

use super::Mapper;

const PRG_BANK_8K: usize = 8 * 1024;
const CHR_BANK_1K: usize = 1024;
const CHR_WINDOW_BYTES: usize = 8 * 1024;
/// 8KB of on-cartridge WRAM, selectable at CPU `$6000..=$7FFF`.
const WRAM_BYTES: usize = 8 * 1024;
const PRG_RAM_BASE: u16 = 0x6000;
const PRG_RAM_END: u16 = 0x7FFF;
/// FME-7 requires at least two 8KB PRG banks (16KB): one switchable and one
/// fixed at `$E000..=$FFFF`.
const MIN_PRG_BANKS: usize = 2;

/// Reg `0x8` bit 6: `$6000..=$7FFF` selects WRAM (1) rather than a PRG-ROM bank (0).
const WRAM_SELECT: u8 = 0x40;
/// Reg `0x8` bit 7: WRAM chip enable (only meaningful when WRAM is selected).
const WRAM_ENABLE: u8 = 0x80;
/// Reg `0x8` bits 0-5: PRG-ROM 8KB bank mapped at `$6000..=$7FFF` when bit 6 = 0.
const WRAM_BANK_MASK: u8 = 0x3F;

/// Reg `0xD` bit 0: IRQ counter enable (counter only decrements when set).
const IRQ_COUNTER_ENABLE: u8 = 0x01;
/// Reg `0xD` bit 7: IRQ enable (IRQ line may assert only when set).
const IRQ_ENABLE: u8 = 0x80;

/// The core pumps `on_ppu_dot` exactly three times per CPU (M2) cycle (see
/// `advance_hardware_cycles` / `apply_dmc_dma_request` in `api.rs`). The FME-7
/// IRQ counter is CPU-cycle clocked, so we divide the dot stream by three.
const DOTS_PER_CPU_CYCLE: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Mapper 69 (Sunsoft FME-7 / 5B): banked PRG/CHR with a 16-bit CPU-cycle IRQ
/// counter and selectable WRAM/PRG at `$6000..=$7FFF`.
///
/// The register interface is a two-step command/parameter pair:
/// * `$8000..=$9FFF` selects an internal register (`value & 0x0F`).
/// * `$A000..=$BFFF` writes a parameter into the selected register.
///
/// `$C000..=$FFFF` drive the 5B sound chip and are intentionally out of scope.
pub struct Fme7 {
    prg_bank_count_8k: usize,
    prg_rom: Vec<u8>,
    chr_bank_count_1k: usize,
    chr_data: Vec<u8>,
    chr_writable: bool,
    /// Currently selected internal register (`0x0..=0xF`), set via `$8000..=$9FFF`.
    command: u8,
    /// Regs `0x0..=0x7`: eight 1KB CHR banks for `$0000,$0400,…,$1C00`.
    chr_banks: [u8; 8],
    /// Reg `0x8`: `$6000..=$7FFF` bank/RAM control (see the `WRAM_*` bit consts).
    wram_control: u8,
    /// Regs `0x9`/`0xA`/`0xB`: 8KB PRG banks for `$8000`/`$A000`/`$C000`.
    prg_banks: [u8; 3],
    /// Reg `0xC`: nametable mirroring.
    mirroring: NametableMirroring,
    /// Reg `0xD`: IRQ control (bit 0 = counter enable, bit 7 = IRQ enable).
    irq_control: u8,
    /// 16-bit down counter (regs `0xE`/`0xF` set the low/high bytes).
    irq_counter: u16,
    /// Latched IRQ line; set on a filtered underflow, cleared by a reg `0xD` write.
    irq_pending: bool,
    /// 8KB WRAM mapped at `$6000..=$7FFF` when reg `0x8` bit 6 = 1.
    wram: Vec<u8>,
    /// Dot-to-CPU-cycle divider. Transient PPU-timing state; excluded from the
    /// save-state snapshot.
    sub_cycle: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Fme7State {
    pub command: u8,
    pub chr_banks: [u8; 8],
    pub wram_control: u8,
    pub prg_banks: [u8; 3],
    pub mirroring: NametableMirroring,
    pub irq_control: u8,
    pub irq_counter: u16,
    pub irq_pending: bool,
    pub wram: Vec<u8>,
}

impl Fme7 {
    /// Builds FME-7 from raw PRG/CHR data.
    ///
    /// Inputs are normalized so mapper operations never panic:
    /// - PRG is zero-padded to at least two 8KB banks and rounded up to a full bank.
    /// - Empty CHR becomes one writable 8KB CHR-RAM window; non-empty CHR is
    ///   padded up to a full 1KB bank (CHR-ROM, not writable).
    #[must_use]
    pub fn from_prg_chr(mut prg_rom: Vec<u8>, chr_rom: Vec<u8>) -> Self {
        let min_prg_bytes = MIN_PRG_BANKS * PRG_BANK_8K;
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
            command: 0,
            chr_banks: [0, 1, 2, 3, 4, 5, 6, 7],
            // Default: WRAM selected + enabled at $6000 (bit6|bit7), like power-on
            // carts that expect their save RAM present.
            wram_control: WRAM_SELECT | WRAM_ENABLE,
            prg_banks: [0, 1, 2],
            mirroring: NametableMirroring::Vertical,
            irq_control: 0,
            irq_counter: 0,
            irq_pending: false,
            wram: vec![0_u8; WRAM_BYTES],
            sub_cycle: 0,
        }
    }

    #[must_use]
    pub(crate) fn state(&self) -> Fme7State {
        Fme7State {
            command: self.command,
            chr_banks: self.chr_banks,
            wram_control: self.wram_control,
            prg_banks: self.prg_banks,
            mirroring: self.mirroring,
            irq_control: self.irq_control,
            irq_counter: self.irq_counter,
            irq_pending: self.irq_pending,
            wram: self.wram.clone(),
        }
    }

    pub(crate) fn restore_state(&mut self, state: Fme7State) {
        self.command = state.command;
        self.chr_banks = state.chr_banks;
        self.wram_control = state.wram_control;
        self.prg_banks = state.prg_banks;
        self.mirroring = state.mirroring;
        self.irq_control = state.irq_control;
        self.irq_counter = state.irq_counter;
        self.irq_pending = state.irq_pending;
        if state.wram.len() == self.wram.len() {
            self.wram.copy_from_slice(&state.wram);
        } else {
            self.wram = state.wram;
            self.wram.resize(WRAM_BYTES, 0);
        }
        // sub_cycle is transient timing state; reset the divider on restore.
        self.sub_cycle = 0;
    }

    #[must_use]
    fn normalize_prg_bank(&self, raw: u8) -> usize {
        usize::from(raw) % self.prg_bank_count_8k.max(1)
    }

    #[must_use]
    fn normalize_chr_bank(&self, raw: u8) -> usize {
        usize::from(raw) % self.chr_bank_count_1k.max(1)
    }

    /// Returns the currently mapped 8KB CHR window, assembled from the eight 1KB
    /// bank registers (`0x0..=0x7`).
    #[must_use]
    pub fn chr_window(&self) -> [u8; CHR_WINDOW_BYTES] {
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        for (slot, &bank) in self.chr_banks.iter().enumerate() {
            let src = self.normalize_chr_bank(bank) * CHR_BANK_1K;
            let dst = slot * CHR_BANK_1K;
            window[dst..dst + CHR_BANK_1K].copy_from_slice(&self.chr_data[src..src + CHR_BANK_1K]);
        }
        window
    }

    /// Returns `true` when mapped CHR should be writable by the PPU (CHR-RAM).
    #[must_use]
    pub fn chr_writable(&self) -> bool {
        self.chr_writable
    }

    /// Synchronizes writable CHR-RAM from the current PPU window, writing each
    /// visible 1KB slot back into its mapped CHR bank.
    pub fn sync_chr_ram_from_ppu_window(&mut self, window: &[u8; CHR_WINDOW_BYTES]) {
        if !self.chr_writable {
            return;
        }
        for (slot, &bank) in self.chr_banks.iter().enumerate() {
            let dst = self.normalize_chr_bank(bank) * CHR_BANK_1K;
            let src = slot * CHR_BANK_1K;
            self.chr_data[dst..dst + CHR_BANK_1K].copy_from_slice(&window[src..src + CHR_BANK_1K]);
        }
    }

    /// Returns the current nametable mirroring mode (reg `0xC`).
    #[must_use]
    pub fn mirroring(&self) -> NametableMirroring {
        self.mirroring
    }

    /// Returns the pending mapper IRQ level.
    #[must_use]
    pub fn irq_pending(&self) -> bool {
        self.irq_pending
    }

    /// Advances the FME-7 IRQ counter for a single PPU dot.
    ///
    /// The counter is CPU-cycle clocked, but this core only exposes a per-PPU-dot
    /// hook. Because `on_ppu_dot` is pumped exactly `DOTS_PER_CPU_CYCLE` times
    /// per CPU cycle, we accumulate dots and clock the counter once every third
    /// call. `scanline`/`dot`/`rendering_enabled`/`ppu_ctrl` are irrelevant to
    /// this counter (it is unrelated to rendering) and are ignored.
    pub fn on_ppu_dot(
        &mut self,
        _scanline: u16,
        _dot: u16,
        _rendering_enabled: bool,
        _ppu_ctrl: u8,
    ) {
        self.sub_cycle += 1;
        if self.sub_cycle >= DOTS_PER_CPU_CYCLE {
            self.sub_cycle = 0;
            self.clock_irq_cpu_cycle();
        }
    }

    /// Performs one CPU-cycle step of the 16-bit down counter.
    ///
    /// When counting is enabled (reg `0xD` bit 0) the counter decrements by one;
    /// on the `$0000 -> $FFFF` underflow it latches `irq_pending` if IRQs are
    /// enabled (reg `0xD` bit 7). The counter keeps running (wraps to `$FFFF`)
    /// regardless, and `irq_pending` stays asserted until acknowledged by a write
    /// to reg `0xD`.
    fn clock_irq_cpu_cycle(&mut self) {
        if self.irq_control & IRQ_COUNTER_ENABLE == 0 {
            return;
        }
        let (next, underflow) = self.irq_counter.overflowing_sub(1);
        self.irq_counter = next;
        if underflow && self.irq_control & IRQ_ENABLE != 0 {
            self.irq_pending = true;
        }
    }

    /// Reads a byte from the `$6000..=$7FFF` window.
    ///
    /// Reg `0x8` bit 6 selects between WRAM (1) and a PRG-ROM bank (0). When WRAM
    /// is selected, bit 7 must also be set for reads to see RAM; otherwise the
    /// region floats and reads back `0xFF` (open-bus approximation). Shared by the
    /// `Mapper::read_prg` `$6000` arm and the CPU-bus `read_prg_ram` route so the
    /// two can never diverge.
    #[must_use]
    fn prg_ram_read(&self, addr: u16) -> u8 {
        let offset = usize::from(addr - PRG_RAM_BASE);
        if self.wram_control & WRAM_SELECT != 0 {
            if self.wram_control & WRAM_ENABLE != 0 {
                self.wram[offset]
            } else {
                0xFF
            }
        } else {
            let bank = self.normalize_prg_bank(self.wram_control & WRAM_BANK_MASK);
            self.prg_rom[bank * PRG_BANK_8K + offset]
        }
    }

    /// Writes a byte into the `$6000..=$7FFF` window. Writes land only when WRAM
    /// is selected (bit 6) and enabled (bit 7); ROM-mapped or disabled writes are
    /// ignored. Shared by the `Mapper::write_prg` `$6000` arm and the CPU-bus
    /// `write_prg_ram` route.
    fn prg_ram_write(&mut self, addr: u16, value: u8) {
        if self.wram_control & WRAM_SELECT != 0 && self.wram_control & WRAM_ENABLE != 0 {
            self.wram[usize::from(addr - PRG_RAM_BASE)] = value;
        }
    }

    /// Reads the `$6000..=$7FFF` window for the CPU bus, or `None` when `addr`
    /// is outside it.
    #[must_use]
    pub fn read_prg_ram(&self, addr: u16) -> Option<u8> {
        (PRG_RAM_BASE..=PRG_RAM_END)
            .contains(&addr)
            .then(|| self.prg_ram_read(addr))
    }

    /// Writes the `$6000..=$7FFF` window from the CPU bus; ignores addresses
    /// outside it.
    pub fn write_prg_ram(&mut self, addr: u16, value: u8) {
        if (PRG_RAM_BASE..=PRG_RAM_END).contains(&addr) {
            self.prg_ram_write(addr, value);
        }
    }

    /// Reads PRG using the FME-7 bank mapping.
    #[must_use]
    pub fn read_prg(&self, addr: u16) -> u8 {
        <Self as Mapper>::read_prg(self, addr)
    }

    /// Writes to the FME-7 register space.
    pub fn write_prg(&mut self, addr: u16, value: u8) {
        <Self as Mapper>::write_prg(self, addr, value);
    }

    /// Applies a parameter write to the currently selected internal register.
    fn write_parameter(&mut self, value: u8) {
        match self.command & 0x0F {
            reg @ 0x0..=0x7 => self.chr_banks[usize::from(reg)] = value,
            0x8 => self.wram_control = value,
            0x9 => self.prg_banks[0] = value,
            0xA => self.prg_banks[1] = value,
            0xB => self.prg_banks[2] = value,
            0xC => {
                self.mirroring = match value & 0x03 {
                    0 => NametableMirroring::Vertical,
                    1 => NametableMirroring::Horizontal,
                    2 => NametableMirroring::OneScreenLower,
                    _ => NametableMirroring::OneScreenUpper,
                };
            }
            0xD => {
                // IRQ control. Writing here also acknowledges any pending IRQ.
                self.irq_control = value;
                self.irq_pending = false;
            }
            0xE => self.irq_counter = (self.irq_counter & 0xFF00) | u16::from(value),
            0xF => self.irq_counter = (self.irq_counter & 0x00FF) | (u16::from(value) << 8),
            _ => unreachable!("command masked to 0x0..=0xF"),
        }
    }
}

impl Mapper for Fme7 {
    fn read_prg(&self, addr: u16) -> u8 {
        if (PRG_RAM_BASE..=PRG_RAM_END).contains(&addr) {
            // $6000-$7FFF: WRAM or a switchable PRG-ROM bank (shared with the
            // CPU-bus read_prg_ram route so behavior can never diverge).
            return self.prg_ram_read(addr);
        }
        if addr < 0x8000 {
            return 0xFF;
        }
        let within = (usize::from(addr) - 0x8000) & 0x1FFF;
        let bank = match (usize::from(addr) - 0x8000) / PRG_BANK_8K {
            0 => self.normalize_prg_bank(self.prg_banks[0]), // $8000-$9FFF (reg 0x9)
            1 => self.normalize_prg_bank(self.prg_banks[1]), // $A000-$BFFF (reg 0xA)
            2 => self.normalize_prg_bank(self.prg_banks[2]), // $C000-$DFFF (reg 0xB)
            _ => self.prg_bank_count_8k - 1,                 // $E000-$FFFF fixed to last
        };
        self.prg_rom[bank * PRG_BANK_8K + within]
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        if (PRG_RAM_BASE..=PRG_RAM_END).contains(&addr) {
            // Shared with the CPU-bus write_prg_ram route.
            self.prg_ram_write(addr, value);
            return;
        }
        match addr {
            // Command register: select which internal register (0x0..=0xF)
            // subsequent parameter writes target.
            0x8000..=0x9FFF => self.command = value & 0x0F,
            // Parameter register: write into the selected internal register.
            0xA000..=0xBFFF => self.write_parameter(value),
            // $C000-$DFFF (audio command) and $E000-$FFFF (audio data) drive the
            // 5B sound chip, which is out of scope: treat as no-ops.
            _ => {}
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

    fn select(m: &mut Fme7, reg: u8, value: u8) {
        m.write_prg(0x8000, reg); // command: select internal register
        m.write_prg(0xA000, value); // parameter: write into it
    }

    #[test]
    fn fme7_prg_banks_switch_and_e000_is_fixed_last() {
        let mut m = Fme7::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
        select(&mut m, 0x9, 3); // $8000 -> bank 3
        select(&mut m, 0xA, 5); // $A000 -> bank 5
        select(&mut m, 0xB, 6); // $C000 -> bank 6
        assert_eq!(m.read_prg(0x8000), 3);
        assert_eq!(m.read_prg(0xA000), 5);
        assert_eq!(m.read_prg(0xC000), 6);
        assert_eq!(m.read_prg(0xE000), 7); // fixed to last bank (8 - 1)
    }

    #[test]
    fn fme7_6000_rom_mapped_returns_selected_bank() {
        let mut m = Fme7::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
        // Reg 8 bit6 = 0 -> $6000 maps PRG-ROM bank (bits 0-5).
        select(&mut m, 0x8, 4);
        assert_eq!(m.read_prg_ram(0x6000), Some(4));
        assert_eq!(m.read_prg(0x6000), 4);
        // Writes to a ROM-mapped region are ignored.
        m.write_prg(0x6000, 0xAA);
        assert_eq!(m.read_prg(0x6000), 4);
    }

    #[test]
    fn fme7_6000_ram_round_trip_and_disable() {
        let mut m = Fme7::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
        // RAM selected (bit6) + enabled (bit7).
        select(&mut m, 0x8, WRAM_SELECT | WRAM_ENABLE);
        m.write_prg_ram(0x6000, 0x5A);
        m.write_prg(0x7FFF, 0x77);
        assert_eq!(m.read_prg_ram(0x6000), Some(0x5A));
        assert_eq!(m.read_prg(0x7FFF), 0x77);

        // RAM selected but disabled (bit7 = 0): reads open-bus, writes ignored.
        select(&mut m, 0x8, WRAM_SELECT);
        assert_eq!(m.read_prg_ram(0x6000), Some(0xFF));
        m.write_prg(0x6000, 0x12); // ignored
        // Re-enable: original byte survived.
        select(&mut m, 0x8, WRAM_SELECT | WRAM_ENABLE);
        assert_eq!(m.read_prg(0x6000), 0x5A);
    }

    #[test]
    fn fme7_chr_windows_assemble_from_1k_banks() {
        let mut m = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(16));
        select(&mut m, 0x0, 10); // $0000 -> 1KB bank 10
        select(&mut m, 0x7, 15); // $1C00 -> 1KB bank 15
        let window = m.chr_window();
        assert_eq!(window[0x0000], 10);
        assert_eq!(window[0x1C00], 15);
    }

    #[test]
    fn fme7_mirroring_register_maps_all_four_modes() {
        let mut m = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
        select(&mut m, 0xC, 0);
        assert_eq!(m.mirroring(), NametableMirroring::Vertical);
        select(&mut m, 0xC, 1);
        assert_eq!(m.mirroring(), NametableMirroring::Horizontal);
        select(&mut m, 0xC, 2);
        assert_eq!(m.mirroring(), NametableMirroring::OneScreenLower);
        select(&mut m, 0xC, 3);
        assert_eq!(m.mirroring(), NametableMirroring::OneScreenUpper);
    }

    /// Drives `count` CPU cycles worth of dots (3 dots each) through the IRQ hook.
    fn pump_cpu_cycles(m: &mut Fme7, count: u32) {
        for _ in 0..count * u32::from(DOTS_PER_CPU_CYCLE) {
            m.on_ppu_dot(0, 0, false, 0);
        }
    }

    #[test]
    fn fme7_irq_fires_on_underflow_after_n_plus_one_cpu_cycles() {
        let mut m = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
        const N: u16 = 8;
        select(&mut m, 0xE, (N & 0xFF) as u8); // counter low
        select(&mut m, 0xF, (N >> 8) as u8); // counter high
        select(&mut m, 0xD, IRQ_COUNTER_ENABLE | IRQ_ENABLE); // enable count + IRQ

        // After exactly N CPU cycles the counter has reached 0 but not underflowed.
        pump_cpu_cycles(&mut m, u32::from(N));
        assert!(!m.irq_pending());
        assert_eq!(m.irq_counter, 0);

        // The (N+1)-th CPU cycle underflows $0000 -> $FFFF and latches the IRQ.
        pump_cpu_cycles(&mut m, 1);
        assert!(m.irq_pending());
        assert_eq!(m.irq_counter, 0xFFFF);

        // Writing reg 0xD acknowledges/clears the pending IRQ.
        select(&mut m, 0xD, IRQ_COUNTER_ENABLE | IRQ_ENABLE);
        assert!(!m.irq_pending());
    }

    #[test]
    fn fme7_counter_disable_freezes_counter() {
        let mut m = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
        select(&mut m, 0xE, 5);
        select(&mut m, 0xF, 0);
        // IRQ enabled but counter disabled (bit0 = 0).
        select(&mut m, 0xD, IRQ_ENABLE);
        pump_cpu_cycles(&mut m, 20);
        assert_eq!(m.irq_counter, 5); // frozen
        assert!(!m.irq_pending());
    }

    #[test]
    fn fme7_irq_disable_suppresses_assertion_on_underflow() {
        let mut m = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
        select(&mut m, 0xE, 2);
        select(&mut m, 0xF, 0);
        // Counter enabled but IRQ disabled (bit7 = 0).
        select(&mut m, 0xD, IRQ_COUNTER_ENABLE);
        pump_cpu_cycles(&mut m, 3); // 2 -> 1 -> 0 -> underflow
        assert_eq!(m.irq_counter, 0xFFFF);
        assert!(!m.irq_pending()); // suppressed
    }

    #[test]
    fn fme7_sub_cycle_divides_dots_by_three() {
        let mut m = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
        select(&mut m, 0xE, 10);
        select(&mut m, 0xF, 0);
        select(&mut m, 0xD, IRQ_COUNTER_ENABLE | IRQ_ENABLE);
        // Two dots is less than one CPU cycle: no decrement yet.
        m.on_ppu_dot(0, 0, false, 0);
        m.on_ppu_dot(0, 0, false, 0);
        assert_eq!(m.irq_counter, 10);
        // The third dot completes one CPU cycle and decrements once.
        m.on_ppu_dot(0, 0, false, 0);
        assert_eq!(m.irq_counter, 9);
    }

    #[test]
    fn fme7_chr_ram_when_chr_absent_is_writable() {
        let mut m = Fme7::from_prg_chr(prg_with_bank_markers(4), vec![]);
        assert!(m.chr_writable());
        let mut window = [0_u8; CHR_WINDOW_BYTES];
        window[0] = 0x42;
        m.sync_chr_ram_from_ppu_window(&window);
        assert_eq!(m.chr_window()[0], 0x42);
    }

    #[test]
    fn fme7_chr_rom_is_not_writable() {
        let m = Fme7::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
        assert!(!m.chr_writable());
    }

    #[test]
    fn fme7_state_round_trips() {
        let mut m = Fme7::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(16));
        select(&mut m, 0x9, 3);
        select(&mut m, 0x2, 11);
        select(&mut m, 0xC, 2);
        select(&mut m, 0x8, WRAM_SELECT | WRAM_ENABLE);
        m.write_prg_ram(0x6000, 0x9A);
        select(&mut m, 0xE, 0x34);
        select(&mut m, 0xF, 0x12);
        select(&mut m, 0xD, IRQ_COUNTER_ENABLE | IRQ_ENABLE);

        let state = m.state();
        assert_eq!(state.irq_counter, 0x1234);
        assert_eq!(state.wram[0], 0x9A);

        let mut restored = Fme7::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(16));
        restored.restore_state(state);
        assert_eq!(restored.read_prg(0x8000), 3);
        assert_eq!(restored.chr_window()[0x0800], 11);
        assert_eq!(restored.mirroring(), NametableMirroring::OneScreenLower);
        assert_eq!(restored.read_prg(0x6000), 0x9A);
        assert_eq!(restored.irq_counter, 0x1234);
    }
}
