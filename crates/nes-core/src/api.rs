//! The core emulator API surface.
//
//! This module provides the primary interface for controlling the NES emulator,
//! injecting inputs, and querying its state. The [`NesCore`] struct is the
//! main entry point for host applications.

use crate::constants::*;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::apu::{Apu, ApuSnapshot, DmcDmaRequest};
use crate::cheat_codes::{CheatCode, CheatCodeError};
use crate::cpu::{Cpu, CpuBusAccess, CpuError, CpuMmioRead, CpuSnapshot, CpuWrite};
use crate::mapper::{
    Axrom, AxromState, Camerica, CamericaState, Cnrom, CnromState, ColorDreams, ColorDreamsState,
    Fme7, Fme7State, Gxrom, GxromState, Mmc1, Mmc1State, Mmc2, Mmc2State, Mmc3, Mmc3State, Mmc4,
    Mmc4State, Namco108, Namco108State, Nrom, Uxrom, UxromState,
};
use crate::ppu::{CHR_FETCH_BUF_LEN, Ppu, PpuSnapshot};
use crate::rom::{NametableMirroring, RomError, parse_ines};
use crate::scheduler::{Scheduler, SchedulerSnapshot};

const DEFAULT_START_PC: u16 = 0xC000;
const DEFAULT_SPEED_PERMILLE: u16 = 1_000;
const BASE_FPS_MILLI: u32 = 60_000;
const PRG_16K_BYTES: usize = 16 * 1024;
const PRG_32K_BYTES: usize = 32 * 1024;
const PRG_8K_BYTES: usize = 8 * 1024;
const PRG_BANK_BYTES: usize = 16 * 1024;
const CHR_8K_BYTES: usize = 8 * 1024;
const CHR_4K_BYTES: usize = 4 * 1024;
const CONTROLLER_OPEN_BUS_MASK: u8 = 0x40;

/// Represents a standard NES controller button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, core::hash::Hash)]
pub enum Button {
    /// A button.
    A,
    /// B button.
    B,
    /// Select button.
    Select,
    /// Start button.
    Start,
    /// Up direction.
    Up,
    /// Down direction.
    Down,
    /// Left direction.
    Left,
    /// Right direction.
    Right,
}

impl Button {
    /// Returns this button's bit in controller bitfields.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::Button;
    /// assert_eq!(Button::A.bit_mask(), 0b0000_0001);
    /// assert_eq!(Button::Start.bit_mask(), 0b0000_1000);
    /// ```
    #[must_use]
    pub fn bit_mask(self) -> u8 {
        match self {
            Self::A => 0b0000_0001,
            Self::B => 0b0000_0010,
            Self::Select => 0b0000_0100,
            Self::Start => 0b0000_1000,
            Self::Up => 0b0001_0000,
            Self::Down => 0b0010_0000,
            Self::Left => 0b0100_0000,
            Self::Right => 0b1000_0000,
        }
    }
}

/// Represents the controller port for a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Player {
    /// Controller port 1.
    One,
    /// Controller port 2.
    Two,
}

impl Player {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ControllerState {
    bits: u8,
    shift: u8,
}

#[derive(Debug, Clone, Default)]
struct ControllerPorts {
    controllers: [ControllerState; 2],
    controller_strobe: bool,
}

impl ControllerPorts {
    fn set_controller_bits(&mut self, bits: u8, player: Player) {
        let state = &mut self.controllers[player.index()];
        state.bits = bits;
        if self.controller_strobe {
            state.shift = bits;
        }
    }

    fn write_controller_strobe(&mut self, value: u8) {
        let next_strobe = value & 1 != 0;
        if next_strobe {
            self.controller_strobe = true;
            for c in &mut self.controllers {
                c.shift = c.bits;
            }
            return;
        }

        if self.controller_strobe {
            for c in &mut self.controllers {
                c.shift = c.bits;
            }
        }
        self.controller_strobe = false;
    }

    fn controller_port_sample(&self, player: Player) -> u8 {
        let state = &self.controllers[player.index()];
        let bit = if self.controller_strobe {
            state.bits & 1
        } else {
            state.shift & 1
        };
        bit | CONTROLLER_OPEN_BUS_MASK
    }

    fn consume_controller_read(&mut self, player: Player) {
        if !self.controller_strobe {
            let state = &mut self.controllers[player.index()];
            state.shift = (state.shift >> 1) | 0x80;
        }
    }
}

/// Commands that can be executed by the [`NesCore`] to change its state
/// or advance the emulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Pause emulation stepping.
    Pause,
    /// Resume emulation stepping.
    Resume,
    /// Reset runtime state while preserving speed setting.
    Reset,
    /// Reset runtime state and restore default speed.
    PowerCycle,
    /// Execute one CPU instruction.
    StepCpu,
    /// Execute until next scanline boundary.
    StepScanline,
    /// Execute until next frame boundary.
    StepFrame,
    /// Replace full controller bitfield.
    SetControllerState(u8),
    /// Replace full controller bitfield for player 2.
    SetController2State(u8),
    /// Press a single controller button.
    PressButton(Button),
    /// Press a single controller button for player 2.
    PressButton2(Button),
    /// Release a single controller button.
    ReleaseButton(Button),
    /// Release a single controller button for player 2.
    ReleaseButton2(Button),
    /// Set speed multiplier in permille (`1000 == 1.0x`).
    SetSpeed(u16),
}

/// Queries that can be sent to the [`NesCore`] to inspect its current state
/// without advancing the emulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreQuery {
    /// Returns paused/speed/controller state.
    EmulatorState,
    /// Returns current CPU registers.
    Registers,
    /// Returns memory-mapped read value for one address.
    Memory(u16),
    /// Returns current target FPS in milli-Hz.
    FpsMilli,
    /// Returns PPU frame counter.
    PpuFrameCounter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Lightweight machine status query response.
pub struct EmulatorState {
    /// Pause state.
    pub paused: bool,
    /// Speed multiplier in permille.
    pub speed_permille: u16,
    /// Latched controller bits.
    pub controller_bits: u8,
    /// Latched controller bits for player 2.
    pub controller2_bits: u8,
}

/// The result of executing a [`CoreQuery`] on the [`NesCore`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryResult {
    /// [`CoreQuery::EmulatorState`] response.
    EmulatorState(EmulatorState),
    /// [`CoreQuery::Registers`] response.
    Registers(Box<CpuSnapshot>),
    /// [`CoreQuery::Memory`] response.
    Memory(u8),
    /// [`CoreQuery::FpsMilli`] response.
    FpsMilli(u32),
    /// [`CoreQuery::PpuFrameCounter`] response.
    PpuFrameCounter(u64),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Complete serializable machine snapshot for save-state support.
pub struct CoreSnapshot {
    /// Pause state.
    pub paused: bool,
    /// Speed multiplier.
    pub speed_permille: u16,
    /// Controller bits.
    pub controller_bits: u8,
    /// Controller bits for player 2.
    pub controller2_bits: u8,
    /// Scheduler counters.
    pub scheduler: SchedulerSnapshot,
    /// PPU state.
    pub ppu: PpuSnapshot,
    /// APU state.
    pub apu: ApuSnapshot,
    /// CPU state.
    pub cpu: CpuSnapshot,
    controller_strobe: bool,
    controller_shift: u8,
    controller2_shift: u8,
    pending_oam_dma_page: Option<u8>,
    mapper: Option<LoadedMapper>,
    cheat_codes: Vec<CheatCode>,
    reset_pc: u16,
}

/// Opaque mapper-runtime delta between two [`CoreSnapshot`]s.
///
/// This captures mutable mapper state such as bank registers, mirroring mode,
/// and IRQ counters without exposing internal mapper implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapperDelta {
    kind: MapperDeltaKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MapperDeltaKind {
    Replace(Option<LoadedMapper>),
    Uxrom(UxromState),
    Mmc1(Mmc1State),
    Cnrom(CnromState),
    Axrom(AxromState),
    Gxrom(GxromState),
    Mmc3(Mmc3State),
    ColorDreams(ColorDreamsState),
    Camerica(CamericaState),
    Namco108(Namco108State),
    Fme7(Fme7State),
    Mmc2(Mmc2State),
    Mmc4(Mmc4State),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Metadata returned after successfully loading a ROM.
pub struct RomLoadInfo {
    /// Mapper ID from iNES header.
    pub mapper_id: u8,
    /// PRG ROM payload size in bytes.
    pub prg_rom_bytes: usize,
    /// Effective reset vector used by the core.
    pub reset_pc: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
enum LoadedMapper {
    Nrom(Nrom),
    Uxrom(Uxrom),
    Mmc1(Mmc1),
    Cnrom(Cnrom),
    Axrom(Axrom),
    Gxrom(Gxrom),
    Mmc3(Mmc3),
    ColorDreams(ColorDreams),
    Camerica(Camerica),
    Namco108(Namco108),
    Fme7(Fme7),
    Mmc2(Mmc2),
    Mmc4(Mmc4),
}

impl LoadedMapper {
    fn read_prg(&self, addr: u16) -> u8 {
        match self {
            Self::Nrom(mapper) => mapper.read_prg(addr),
            Self::Uxrom(mapper) => mapper.read_prg(addr),
            Self::Mmc1(mapper) => mapper.read_prg(addr),
            Self::Cnrom(mapper) => mapper.read_prg(addr),
            Self::Axrom(mapper) => mapper.read_prg(addr),
            Self::Gxrom(mapper) => mapper.read_prg(addr),
            Self::Mmc3(mapper) => mapper.read_prg(addr),
            Self::ColorDreams(mapper) => mapper.read_prg(addr),
            Self::Camerica(mapper) => mapper.read_prg(addr),
            Self::Namco108(mapper) => mapper.read_prg(addr),
            Self::Fme7(mapper) => mapper.read_prg(addr),
            Self::Mmc2(mapper) => mapper.read_prg(addr),
            Self::Mmc4(mapper) => mapper.read_prg(addr),
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        match self {
            Self::Nrom(mapper) => mapper.write_prg(addr, value),
            Self::Uxrom(mapper) => mapper.write_prg(addr, value),
            Self::Mmc1(mapper) => mapper.write_prg(addr, value),
            Self::Cnrom(mapper) => mapper.write_prg(addr, value),
            Self::Axrom(mapper) => mapper.write_prg(addr, value),
            Self::Gxrom(mapper) => mapper.write_prg(addr, value),
            Self::Mmc3(mapper) => mapper.write_prg(addr, value),
            Self::ColorDreams(mapper) => mapper.write_prg(addr, value),
            Self::Camerica(mapper) => mapper.write_prg(addr, value),
            Self::Namco108(mapper) => mapper.write_prg(addr, value),
            Self::Fme7(mapper) => mapper.write_prg(addr, value),
            Self::Mmc2(mapper) => mapper.write_prg(addr, value),
            Self::Mmc4(mapper) => mapper.write_prg(addr, value),
        }
    }

    /// Reads a byte from cartridge PRG-RAM (`$6000..=$7FFF`), or `None` when the
    /// mapper has no work RAM at `addr`. Mappers without PRG-RAM return `None`
    /// so the CPU flat image at `$6000..=$7FFF` is left untouched.
    fn read_prg_ram(&self, addr: u16) -> Option<u8> {
        match self {
            Self::Mmc3(mapper) => mapper.read_prg_ram(addr),
            Self::Fme7(mapper) => mapper.read_prg_ram(addr),
            Self::Mmc4(mapper) => mapper.read_prg_ram(addr),
            _ => None,
        }
    }

    /// Writes a byte to cartridge PRG-RAM (`$6000..=$7FFF`). Mappers without
    /// work RAM ignore the write.
    fn write_prg_ram(&mut self, addr: u16, value: u8) {
        match self {
            Self::Mmc3(mapper) => mapper.write_prg_ram(addr, value),
            Self::Fme7(mapper) => mapper.write_prg_ram(addr, value),
            Self::Mmc4(mapper) => mapper.write_prg_ram(addr, value),
            _ => {}
        }
    }

    fn chr_window(&self) -> Option<([u8; CHR_8K_BYTES], bool)> {
        match self {
            Self::Mmc3(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            Self::Cnrom(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            Self::Gxrom(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            Self::ColorDreams(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            Self::Camerica(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            Self::Namco108(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            Self::Fme7(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            Self::Mmc2(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            Self::Mmc4(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            _ => None,
        }
    }

    fn mirroring_override(&self) -> Option<NametableMirroring> {
        match self {
            Self::Axrom(mapper) => Some(mapper.mirroring()),
            Self::Mmc3(mapper) => Some(mapper.mirroring()),
            Self::Fme7(mapper) => Some(mapper.mirroring()),
            Self::Mmc2(mapper) => Some(mapper.mirroring()),
            Self::Mmc4(mapper) => Some(mapper.mirroring()),
            _ => None,
        }
    }

    /// Whether this mapper wants per-pattern-fetch CHR notifications (the
    /// MMC2/MMC4 tile latch). Used to gate PPU CHR-fetch recording so
    /// non-latching mappers pay zero cost in the render path.
    #[must_use]
    fn wants_chr_fetch_notify(&self) -> bool {
        matches!(self, Self::Mmc2(_) | Self::Mmc4(_))
    }

    /// Forwards a PPU pattern-table fetch address to the mapper's CHR latch.
    /// Returns `true` when the mapper's CHR window changed and must be re-synced
    /// to the PPU. All non-latching mappers are a no-op returning `false`.
    #[must_use]
    fn notify_ppu_chr_fetch(&mut self, addr: u16) -> bool {
        match self {
            Self::Mmc2(mapper) => mapper.notify_ppu_chr_fetch(addr),
            Self::Mmc4(mapper) => mapper.notify_ppu_chr_fetch(addr),
            _ => false,
        }
    }

    fn irq_pending(&self) -> bool {
        match self {
            Self::Mmc3(mapper) => mapper.irq_pending(),
            Self::Fme7(mapper) => mapper.irq_pending(),
            _ => false,
        }
    }

    fn on_ppu_dot(&mut self, scanline: u16, dot: u16, rendering_enabled: bool, ppu_ctrl: u8) {
        match self {
            Self::Mmc3(mapper) => mapper.on_ppu_dot(scanline, dot, rendering_enabled, ppu_ctrl),
            Self::Fme7(mapper) => mapper.on_ppu_dot(scanline, dot, rendering_enabled, ppu_ctrl),
            _ => {}
        }
    }

    fn sync_chr_ram_from_ppu_window(&mut self, window: &[u8; CHR_8K_BYTES]) {
        match self {
            Self::Cnrom(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            Self::Gxrom(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            Self::Mmc3(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            Self::ColorDreams(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            Self::Camerica(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            Self::Namco108(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            Self::Fme7(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            Self::Mmc2(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            Self::Mmc4(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            _ => {}
        }
    }

    #[must_use]
    fn chr_writable(&self) -> bool {
        match self {
            Self::Cnrom(mapper) => mapper.chr_writable(),
            Self::Gxrom(mapper) => mapper.chr_writable(),
            Self::Mmc3(mapper) => mapper.chr_writable(),
            Self::ColorDreams(mapper) => mapper.chr_writable(),
            Self::Camerica(mapper) => mapper.chr_writable(),
            Self::Namco108(mapper) => mapper.chr_writable(),
            Self::Fme7(mapper) => mapper.chr_writable(),
            Self::Mmc2(mapper) => mapper.chr_writable(),
            Self::Mmc4(mapper) => mapper.chr_writable(),
            _ => false,
        }
    }

    fn delta_to(&self, after: &Self) -> Option<MapperDelta> {
        let kind = match (self, after) {
            (Self::Nrom(_), Self::Nrom(_)) => return None,
            (Self::Uxrom(before), Self::Uxrom(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Uxrom(state))
            }
            (Self::Mmc1(before), Self::Mmc1(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Mmc1(state))
            }
            (Self::Cnrom(before), Self::Cnrom(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Cnrom(state))
            }
            (Self::Axrom(before), Self::Axrom(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Axrom(state))
            }
            (Self::Gxrom(before), Self::Gxrom(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Gxrom(state))
            }
            (Self::Mmc3(before), Self::Mmc3(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Mmc3(state))
            }
            (Self::ColorDreams(before), Self::ColorDreams(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::ColorDreams(state))
            }
            (Self::Camerica(before), Self::Camerica(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Camerica(state))
            }
            (Self::Namco108(before), Self::Namco108(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Namco108(state))
            }
            (Self::Fme7(before), Self::Fme7(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Fme7(state))
            }
            (Self::Mmc2(before), Self::Mmc2(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Mmc2(state))
            }
            (Self::Mmc4(before), Self::Mmc4(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Mmc4(state))
            }
            _ => Some(MapperDeltaKind::Replace(Some(after.clone()))),
        }?;

        Some(MapperDelta { kind })
    }

    #[must_use]
    fn snapshot_delta(&self) -> Option<MapperDelta> {
        let kind = match self {
            Self::Nrom(_) => return None,
            Self::Uxrom(mapper) => MapperDeltaKind::Uxrom(mapper.state()),
            Self::Mmc1(mapper) => MapperDeltaKind::Mmc1(mapper.state()),
            Self::Cnrom(mapper) => MapperDeltaKind::Cnrom(mapper.state()),
            Self::Axrom(mapper) => MapperDeltaKind::Axrom(mapper.state()),
            Self::Gxrom(mapper) => MapperDeltaKind::Gxrom(mapper.state()),
            Self::Mmc3(mapper) => MapperDeltaKind::Mmc3(mapper.state()),
            Self::ColorDreams(mapper) => MapperDeltaKind::ColorDreams(mapper.state()),
            Self::Camerica(mapper) => MapperDeltaKind::Camerica(mapper.state()),
            Self::Namco108(mapper) => MapperDeltaKind::Namco108(mapper.state()),
            Self::Fme7(mapper) => MapperDeltaKind::Fme7(mapper.state()),
            Self::Mmc2(mapper) => MapperDeltaKind::Mmc2(mapper.state()),
            Self::Mmc4(mapper) => MapperDeltaKind::Mmc4(mapper.state()),
        };

        Some(MapperDelta { kind })
    }

    fn apply_delta(&mut self, delta: &MapperDelta, chr_window: &[u8; CHR_8K_BYTES]) {
        match &delta.kind {
            MapperDeltaKind::Uxrom(state) => {
                let Self::Uxrom(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(*state);
            }
            MapperDeltaKind::Mmc1(state) => {
                let Self::Mmc1(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(*state);
            }
            MapperDeltaKind::Cnrom(state) => {
                let Self::Cnrom(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(*state);
            }
            MapperDeltaKind::Axrom(state) => {
                let Self::Axrom(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(*state);
            }
            MapperDeltaKind::Gxrom(state) => {
                let Self::Gxrom(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(*state);
            }
            MapperDeltaKind::Mmc3(state) => {
                let Self::Mmc3(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(state.clone());
            }
            MapperDeltaKind::ColorDreams(state) => {
                let Self::ColorDreams(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(*state);
            }
            MapperDeltaKind::Camerica(state) => {
                let Self::Camerica(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(*state);
            }
            MapperDeltaKind::Namco108(state) => {
                let Self::Namco108(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(*state);
            }
            MapperDeltaKind::Fme7(state) => {
                let Self::Fme7(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(state.clone());
            }
            MapperDeltaKind::Mmc2(state) => {
                let Self::Mmc2(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(*state);
            }
            MapperDeltaKind::Mmc4(state) => {
                let Self::Mmc4(mapper) = self else {
                    debug_assert!(false, "mapper delta kind must match mapper variant");
                    return;
                };
                mapper.restore_state(state.clone());
            }
            MapperDeltaKind::Replace(_) => {
                debug_assert!(
                    false,
                    "replacement mapper deltas are handled by CoreSnapshot"
                );
                return;
            }
        }

        self.sync_chr_ram_from_ppu_window(chr_window);
    }
}

impl CoreSnapshot {
    /// Computes the mapper-specific delta needed to transform this snapshot into `after`.
    #[must_use]
    pub fn mapper_delta(&self, after: &Self) -> Option<MapperDelta> {
        match (&self.mapper, &after.mapper) {
            (None, None) => None,
            (Some(before), Some(after_mapper)) => before.delta_to(after_mapper).or_else(|| {
                (self.ppu.chr != after.ppu.chr && after_mapper.chr_writable())
                    .then(|| after_mapper.snapshot_delta())
                    .flatten()
            }),
            _ => Some(MapperDelta {
                kind: MapperDeltaKind::Replace(after.mapper.clone()),
            }),
        }
    }

    /// Applies a mapper-specific delta to this snapshot in-place.
    pub fn apply_mapper_delta(&mut self, delta: &MapperDelta) {
        match &delta.kind {
            MapperDeltaKind::Replace(mapper) => {
                self.mapper = mapper.clone();
            }
            _ => {
                if let Some(mapper) = self.mapper.as_mut() {
                    mapper.apply_delta(delta, &self.ppu.chr);
                } else {
                    debug_assert!(
                        false,
                        "non-replacement mapper delta requires existing mapper"
                    );
                }
            }
        }
    }
}

/// The central NES emulator state machine.
///
/// `NesCore` manages the execution of the CPU, PPU, and APU, synchronizing
/// their clocks and handling memory access between them. Host applications
/// use this struct to load ROMs, advance emulation frames, and extract
/// video/audio outputs.
///
/// ## Panics
///
/// The example below uses [`Result::unwrap`] for brevity. In a real application,
/// errors from [`NesCore::load_ines_rom`] and [`NesCore::execute`] should be
/// handled properly. Unwrapping a [`CoreError::RomLoadFailed`] or
/// [`CoreError::InvalidSpeed`] will cause a panic.
///
/// ## Examples
///
/// ```
/// use nes_core::{NesCore, Command, Button};
///
/// let mut core = NesCore::new();
/// // Load a minimal dummy ROM (normally you'd load a real .nes file)
/// let mut dummy_rom = vec![
///     0x4E, 0x45, 0x53, 0x1A, // "NES\x1A"
///     0x01, 0x01, 0x00, 0x00, // 16KB PRG, 8KB CHR, NROM
///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Padding
/// ];
/// // Append 16KB PRG ROM and 8KB CHR ROM to make it a valid cartridge
/// dummy_rom.extend(vec![0x00; 16 * 1024 + 8 * 1024]);
/// core.load_ines_rom(&dummy_rom).unwrap();
///
/// // Execute commands to drive the core
/// core.execute(Command::StepFrame).unwrap();
/// core.execute(Command::PressButton(Button::A)).unwrap();
///
/// // Extract framebuffer for rendering
/// let frame = core.framebuffer_rgba();
/// assert_eq!(frame.len(), 256 * 240 * 4);
/// ```
#[derive(Debug, Clone)]
pub struct NesCore {
    paused: bool,
    speed_permille: u16,
    scheduler: Scheduler,
    ppu: Ppu,
    ports: ControllerPorts,
    mapper: Option<LoadedMapper>,
    cheat_codes: Vec<CheatCode>,
    reset_pc: u16,
    cpu: Cpu,
    apu: Apu,
    pending_oam_dma_page: Option<u8>,
    last_cpu_trace: Option<String>,
    last_cpu_bus_trace: Vec<CpuBusAccess>,
    scratch_writes: Vec<CpuWrite>,
    scratch_mmio_reads: Vec<CpuMmioRead>,
}

/// Errors that can occur when interacting with the [`NesCore`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreError {
    /// Command is not currently supported by the runtime mode.
    UnsupportedCommand,
    /// Speed value was zero.
    InvalidSpeed(u16),
    /// ROM parse/load failed.
    RomLoadFailed(RomError),
    /// CPU stepping failed.
    CpuStepFailed(CpuError),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCommand => f.write_str("unsupported command"),
            Self::InvalidSpeed(speed) => write!(f, "invalid speed multiplier: {speed}"),
            Self::RomLoadFailed(err) => write!(f, "rom load failed: {err}"),
            Self::CpuStepFailed(err) => write!(f, "cpu step failed: {err}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl NesCore {
    /// Creates a new core with power-on defaults and no loaded mapper.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::NesCore;
    /// let core = NesCore::new();
    /// assert!(!core.is_paused());
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self {
            paused: false,
            speed_permille: DEFAULT_SPEED_PERMILLE,
            scheduler: Scheduler::new(),
            ppu: Ppu::new(),
            ports: ControllerPorts::default(),
            mapper: None,
            cheat_codes: Vec::new(),
            reset_pc: DEFAULT_START_PC,
            cpu: Cpu::new(DEFAULT_START_PC),
            apu: Apu::new(),
            pending_oam_dma_page: None,
            last_cpu_trace: None,
            last_cpu_bus_trace: Vec::new(),
            scratch_writes: Vec::new(),
            scratch_mmio_reads: Vec::new(),
        }
    }

    /// Returns whether stepping is currently paused.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Returns current speed multiplier in permille (`1000 == 1.0x`).
    #[must_use]
    pub const fn speed_permille(&self) -> u16 {
        self.speed_permille
    }

    /// Returns total CPU cycles observed by the scheduler.
    #[must_use]
    pub fn total_cycles(&self) -> u64 {
        self.scheduler.total_cycles()
    }

    /// Returns current controller bitfield.
    #[must_use]
    pub fn controller_bits(&self) -> u8 {
        self.ports.controllers[0].bits
    }

    /// Returns current controller bitfield for player 2.
    #[must_use]
    pub fn controller2_bits(&self) -> u8 {
        self.ports.controllers[1].bits
    }

    /// Loads raw bytes directly into the CPU's memory image.
    ///
    /// This bypasses standard cartridge loading and mapper interception, allowing
    /// you to construct synthetic memory environments. It is extremely useful for
    /// writing isolated instruction tests, executing headless machine code snippets,
    /// or bootstrapping a minimal runtime without needing a full `.nes` file header.
    ///
    /// When writing into PRG space (`>= 0x8000`), any previously active mapper state
    /// is intentionally cleared and the default reset vector is reset. This guarantees
    /// that raw memory execution semantics remain explicitly predictable.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::{NesCore, Command};
    ///
    /// let mut core = NesCore::new();
    /// // Load a simple program: LDA #$42
    /// core.load_cpu_bytes(0xC000, &[0xA9, 0x42]);
    ///
    /// // Execute a single instruction
    /// core.execute(Command::StepCpu).unwrap();
    ///
    /// // The accumulator should now hold the loaded literal
    /// assert_eq!(core.cpu_a(), 0x42);
    /// ```
    pub fn load_cpu_bytes(&mut self, start: u16, bytes: &[u8]) {
        if start >= 0x8000 {
            self.mapper = None;
            self.reset_pc = DEFAULT_START_PC;
        }
        self.cpu.load_bytes(start, bytes);
    }

    /// Returns CPU program counter.
    #[must_use]
    pub fn cpu_pc(&self) -> u16 {
        self.cpu.pc()
    }

    /// Returns CPU accumulator.
    #[must_use]
    pub fn cpu_a(&self) -> u8 {
        self.cpu.a()
    }

    /// Returns CPU X register.
    #[must_use]
    pub fn cpu_x(&self) -> u8 {
        self.cpu.x()
    }

    /// Enables or disables CPU trace generation.
    ///
    /// Defaults to `true` in debug builds and `false` in release builds.
    /// Override to `true` in release if you need the disassembly view (debugger,
    /// step-cpu UI), or to `false` in debug for throughput profiling.
    pub fn set_trace_enabled(&mut self, enabled: bool) {
        self.cpu.set_trace_enabled(enabled);
    }

    /// Returns last executed instruction trace string, if any.
    #[must_use]
    pub fn last_cpu_trace(&self) -> Option<&str> {
        self.last_cpu_trace.as_deref()
    }

    /// Returns CPU register snapshot.
    #[must_use]
    pub fn cpu_snapshot(&self) -> CpuSnapshot {
        self.cpu.snapshot()
    }

    /// Returns last instruction's bus access trace entries.
    #[must_use]
    pub fn last_cpu_bus_trace(&self) -> &[CpuBusAccess] {
        &self.last_cpu_bus_trace
    }

    /// Reads CPU-visible memory with MMIO-aware behavior.
    #[must_use]
    pub fn read_memory(&self, addr: u16) -> u8 {
        match addr {
            0x2002 => self.ppu.status(),
            0x2004 => self.ppu.peek_oam_data_for_cpu_read(),
            0x2007 => self.ppu.peek_data_for_cpu_read(),
            0x4015 => self.apu.peek_status(),
            0x4016 => self.ports.controller_port_sample(Player::One),
            0x4017 => self.ports.controller_port_sample(Player::Two),
            _ => self.cpu.read_byte(addr),
        }
    }

    /// Reads `$4015` APU status and mirrors the result into CPU memory.
    pub fn read_apu_status(&mut self) -> u8 {
        let status = self.apu.read_status();
        self.cpu.write_byte(0x4015, status);
        status
    }

    /// Returns the currently configured cheat codes.
    #[must_use]
    pub fn cheat_codes(&self) -> &[CheatCode] {
        &self.cheat_codes
    }

    /// Adds one cheat code and reapplies PRG mappings immediately.
    ///
    /// # Errors
    ///
    /// Returns [`CheatCodeError`] when the code string is not a valid NES
    /// cheat code.
    pub fn add_cheat_code(&mut self, raw: &str) -> Result<CheatCode, CheatCodeError> {
        let code: CheatCode = raw.parse()?;
        self.cheat_codes.push(code.clone());
        if self.mapper.is_some() {
            self.sync_mapper_prg_window();
        }
        Ok(code)
    }

    /// Removes all configured cheat codes and restores the mapper PRG view.
    pub fn clear_cheat_codes(&mut self) {
        if self.cheat_codes.is_empty() {
            return;
        }
        self.cheat_codes.clear();
        if self.mapper.is_some() {
            self.sync_mapper_prg_window();
        }
    }

    /// Writes to CPU bus then applies mapped side-effects immediately.
    pub fn write_cpu_bus(&mut self, addr: u16, value: u8) {
        self.cpu.write_byte(addr, value);
        self.apply_cpu_write_side_effect(addr, value);
        self.sync_ppu_register_image();
    }

    /// Returns PPU frame counter.
    #[must_use]
    pub fn ppu_frame_counter(&self) -> u64 {
        self.ppu.frame_counter()
    }

    /// Returns current PPU scanline.
    #[must_use]
    pub fn ppu_scanline(&self) -> u16 {
        self.ppu.scanline()
    }

    /// Returns current PPU dot.
    #[must_use]
    pub fn ppu_dot(&self) -> u16 {
        self.ppu.dot()
    }

    /// Returns total PPU cycles from scheduler.
    #[must_use]
    pub fn ppu_total_cycles(&self) -> u64 {
        self.scheduler.ppu_cycles()
    }

    /// Returns total APU cycles from scheduler.
    #[must_use]
    pub fn apu_total_cycles(&self) -> u64 {
        self.scheduler.apu_cycles()
    }

    /// Returns APU quarter-frame tick counter.
    #[must_use]
    pub fn apu_quarter_frame_ticks(&self) -> u64 {
        self.apu.quarter_frame_ticks()
    }

    /// Returns APU half-frame tick counter.
    #[must_use]
    pub fn apu_half_frame_ticks(&self) -> u64 {
        self.apu.half_frame_ticks()
    }

    /// Returns whether any APU IRQ source is pending.
    #[must_use]
    pub fn apu_irq_pending(&self) -> bool {
        self.apu.irq_pending()
    }

    /// Returns whether DMC IRQ is pending.
    #[must_use]
    pub fn apu_dmc_irq_pending(&self) -> bool {
        self.apu.dmc_irq_pending()
    }

    /// Returns remaining DMC sample bytes.
    #[must_use]
    pub fn apu_dmc_bytes_remaining(&self) -> u16 {
        self.apu.dmc_bytes_remaining()
    }

    /// Returns DMC memory fetch count.
    #[must_use]
    pub fn apu_dmc_fetch_count(&self) -> u64 {
        self.apu.dmc_fetch_count()
    }

    /// Returns pulse timer reloads for debug/metrics inspection.
    #[must_use]
    pub fn apu_pulse_timer_reloads(&self) -> (u16, u16) {
        self.apu.pulse_timer_reloads()
    }

    /// Reads one OAM byte by index.
    #[must_use]
    pub fn ppu_oam_byte(&self, index: u8) -> u8 {
        self.ppu.oam_byte(index)
    }

    /// Returns target frames-per-second as milli-Hz.
    #[must_use]
    pub fn fps_milli(&self) -> u32 {
        BASE_FPS_MILLI.saturating_mul(self.speed_permille as u32) / DEFAULT_SPEED_PERMILLE as u32
    }

    /// Returns a freshly allocated RGBA framebuffer snapshot.
    #[must_use]
    pub fn framebuffer_rgba(&self) -> Vec<u8> {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        self.fill_framebuffer_rgba(&mut frame);
        frame
    }

    /// Writes current RGBA framebuffer into caller-provided buffer.
    pub fn fill_framebuffer_rgba(&self, frame: &mut [u8]) {
        self.ppu.render_rgba(frame);
    }

    /// Drains one host-frame-sized audio chunk (`AUDIO_CHUNK_SAMPLES`).
    #[must_use]
    pub fn audio_chunk_i16(&mut self) -> Vec<i16> {
        let mut buffer = vec![0; AUDIO_CHUNK_SAMPLES];
        self.fill_audio_chunk_i16(&mut buffer);
        buffer
    }

    /// Fills caller-provided audio buffer with drained APU samples.
    pub fn fill_audio_chunk_i16(&mut self, samples: &mut [i16]) {
        self.apu.fill_samples(samples, self.paused);
    }

    /// Returns a compact hash of emulation state for regression checks.
    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let paused = if self.paused { 1_u64 } else { 0_u64 };
        let cpu = self.cpu.snapshot();
        paused
            ^ self.scheduler.cpu_cycles().rotate_left(13)
            ^ self.scheduler.ppu_cycles().rotate_left(29)
            ^ self.scheduler.apu_cycles().rotate_left(47)
            ^ (self.speed_permille as u64).rotate_left(3)
            ^ (self.ports.controllers[0].bits as u64).rotate_left(7)
            ^ (self.ports.controllers[1].bits as u64).rotate_left(27)
            ^ ((if self.ports.controller_strobe {
                1_u64
            } else {
                0_u64
            })
            .rotate_left(15))
            ^ (self.ports.controllers[0].shift as u64).rotate_left(21)
            ^ (self.ports.controllers[1].shift as u64).rotate_left(33)
            ^ (cpu.pc as u64).rotate_left(19)
            ^ (cpu.a as u64).rotate_left(23)
            ^ (cpu.x as u64).rotate_left(31)
            ^ (cpu.y as u64).rotate_left(37)
            ^ (cpu.status as u64).rotate_left(41)
            ^ self.ppu.frame_counter().rotate_left(11)
            ^ (self.ppu.status() as u64).rotate_left(17)
            ^ self.apu.total_cycles().rotate_left(59)
            ^ self.apu.quarter_frame_ticks().rotate_left(5)
            ^ self.apu.half_frame_ticks().rotate_left(9)
            ^ ((if self.apu.irq_pending() { 1_u64 } else { 0_u64 }).rotate_left(57))
            ^ self.cheat_code_hash_component().rotate_left(25)
            ^ self.mapper_hash_component().rotate_left(53)
    }

    /// Captures full core save-state snapshot.
    /// Creates a complete snapshot of the emulation state for save states or rollback.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::NesCore;
    /// let core = NesCore::new();
    /// let snap = core.save_state();
    /// ```
    #[must_use]
    pub fn save_state(&self) -> CoreSnapshot {
        CoreSnapshot {
            paused: self.paused,
            speed_permille: self.speed_permille,
            controller_bits: self.ports.controllers[0].bits,
            controller2_bits: self.ports.controllers[1].bits,
            scheduler: self.scheduler.snapshot(),
            ppu: self.ppu.snapshot(),
            apu: self.apu.snapshot(),
            cpu: self.cpu.snapshot(),
            controller_strobe: self.ports.controller_strobe,
            controller_shift: self.ports.controllers[0].shift,
            controller2_shift: self.ports.controllers[1].shift,
            pending_oam_dma_page: self.pending_oam_dma_page,
            mapper: self.mapper.clone(),
            cheat_codes: self.cheat_codes.clone(),
            reset_pc: self.reset_pc,
        }
    }

    /// Restores full core save-state snapshot.
    /// Restores the emulator from a previously created [`CoreSnapshot`].
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::NesCore;
    /// let mut core = NesCore::new();
    /// let snap = core.save_state();
    /// core.load_state(&snap);
    /// ```
    pub fn load_state(&mut self, snapshot: &CoreSnapshot) {
        self.paused = snapshot.paused;
        self.speed_permille = snapshot.speed_permille;
        self.ports.controllers[0].bits = snapshot.controller_bits;
        self.ports.controllers[1].bits = snapshot.controller2_bits;
        self.scheduler.restore(snapshot.scheduler);
        self.ppu.restore(snapshot.ppu.clone());
        self.apu.restore(snapshot.apu.clone());
        self.cpu.restore(snapshot.cpu);
        self.ports.controller_strobe = snapshot.controller_strobe;
        self.ports.controllers[0].shift = snapshot.controller_shift;
        self.ports.controllers[1].shift = snapshot.controller2_shift;
        self.pending_oam_dma_page = snapshot.pending_oam_dma_page;
        self.mapper = snapshot.mapper.clone();
        self.cheat_codes = snapshot.cheat_codes.clone();
        self.reset_pc = snapshot.reset_pc;
        self.sync_mapper_prg_window();
        self.sync_mapper_chr_window();
        self.sync_mapper_mirroring();
        self.sync_ppu_register_image();
        self.last_cpu_trace = None;
        self.last_cpu_bus_trace.clear();
        self.scratch_writes.clear();
    }

    /// Replays a command stream identically on this core.
    ///
    /// Useful for recording and reproducing macros or input sequences.
    ///
    /// # Errors
    ///
    /// Propagates the first command execution failure.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::{NesCore, Command};
    /// let mut core = NesCore::new();
    /// // Normally, you'd load a ROM first:
    /// // core.load_ines_rom(&rom_bytes).unwrap();
    /// // core.replay(&[Command::StepFrame, Command::StepFrame]).unwrap();
    /// ```
    pub fn replay(&mut self, commands: &[Command]) -> Result<(), CoreError> {
        for command in commands {
            self.execute(*command)?;
        }
        Ok(())
    }

    /// Parses and loads an iNES format ROM from bytes.
    ///
    /// This sets up the internal mapper and memory mappings, preparing the
    /// emulator to execute code.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::RomLoadFailed`] when parsing or mapper validation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use nes_core::{NesCore, Command};
    ///
    /// let mut core = NesCore::new();
    ///
    /// // Construct a minimal 16-byte iNES header for a 16KB PRG ROM (NROM)
    /// let mut rom = vec![
    ///     0x4E, 0x45, 0x53, 0x1A, // "NES\x1A"
    ///     0x01,                   // 1x 16KB PRG
    ///     0x00,                   // 0x 8KB CHR (Uses CHR-RAM)
    ///     0x00, 0x00, 0x00, 0x00, // Flags
    ///     0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    /// ];
    ///
    /// // Append the actual PRG ROM bytes
    /// rom.extend(vec![0x00; 16 * 1024]);
    ///
    /// // Load the constructed ROM
    /// let info = core.load_ines_rom(&rom).unwrap();
    /// assert_eq!(info.mapper_id, 0);
    /// assert_eq!(info.prg_rom_bytes, 16 * 1024);
    /// ```
    pub fn load_ines_rom(&mut self, rom_bytes: &[u8]) -> Result<RomLoadInfo, CoreError> {
        let rom = parse_ines(rom_bytes).map_err(CoreError::RomLoadFailed)?;
        let mapper = self.build_mapper(rom.mapper_id, rom.prg_rom, rom.chr_rom, rom.mirroring)?;
        self.mapper = Some(mapper);
        self.ppu.load_cartridge(rom.chr_rom, rom.mirroring);
        self.sync_mapper_prg_window();
        self.sync_mapper_chr_window();
        self.sync_mapper_mirroring();

        let reset_pc = {
            let lo = self.cpu.read_byte(0xFFFC);
            let hi = self.cpu.read_byte(0xFFFD);
            let pc = u16::from_le_bytes([lo, hi]);
            if pc == 0 { 0x8000 } else { pc }
        };
        self.reset_pc = reset_pc;

        self.paused = false;
        self.ports.controllers = [ControllerState::default(); 2];
        self.ports.controller_strobe = false;
        self.scheduler.reset();
        self.ppu.reset();
        self.apu.reset();
        self.cpu.reset(reset_pc);
        self.pending_oam_dma_page = None;
        self.sync_ppu_register_image();
        self.last_cpu_trace = None;
        self.last_cpu_bus_trace.clear();
        self.scratch_writes.clear();

        Ok(RomLoadInfo {
            mapper_id: rom.mapper_id,
            prg_rom_bytes: rom.prg_rom.len(),
            reset_pc,
        })
    }

    /// Executes a single control or input command.
    ///
    /// This is the primary way to step the emulator forward, reset it,
    /// or pass inputs. See [`Command`] for available actions.
    ///
    /// # Errors
    ///
    /// Returns a [`CoreError`] when command preconditions fail or stepping fails.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::{NesCore, Command, Button};
    /// let mut core = NesCore::new();
    /// // Normally you load a ROM first before stepping, but we can safely press buttons
    /// core.execute(Command::PressButton(Button::A)).unwrap();
    /// ```
    pub fn execute(&mut self, command: Command) -> Result<(), CoreError> {
        let (player, bits) = match command {
            Command::SetControllerState(bits) => (Player::One, bits),
            Command::SetController2State(bits) => (Player::Two, bits),
            Command::PressButton(button) => (
                Player::One,
                self.ports.controllers[0].bits | button.bit_mask(),
            ),
            Command::PressButton2(button) => (
                Player::Two,
                self.ports.controllers[1].bits | button.bit_mask(),
            ),
            Command::ReleaseButton(button) => (
                Player::One,
                self.ports.controllers[0].bits & !button.bit_mask(),
            ),
            Command::ReleaseButton2(button) => (
                Player::Two,
                self.ports.controllers[1].bits & !button.bit_mask(),
            ),
            Command::Pause => {
                self.paused = true;
                return Ok(());
            }
            Command::Resume => {
                self.paused = false;
                return Ok(());
            }
            Command::Reset => {
                self.reset_runtime();
                return Ok(());
            }
            Command::PowerCycle => {
                self.reset_runtime();
                self.speed_permille = DEFAULT_SPEED_PERMILLE;
                return Ok(());
            }
            Command::StepCpu => {
                self.step_single_instruction()?;
                return Ok(());
            }
            Command::StepScanline => {
                self.step_until_next_scanline()?;
                return Ok(());
            }
            Command::StepFrame => {
                self.step_until_next_frame()?;
                return Ok(());
            }
            Command::SetSpeed(speed) => {
                if speed == 0 {
                    return Err(CoreError::InvalidSpeed(speed));
                }
                self.speed_permille = speed;
                return Ok(());
            }
        };

        self.ports.set_controller_bits(bits, player);
        self.sync_ppu_register_image();
        Ok(())
    }

    /// Executes a readonly query against current state.
    /// Performs a non-mutating query against the emulator state.
    ///
    /// Useful for extracting out-of-band information, like [`EmulatorState`]
    /// or reading mapped memory addresses without modifying cycles.
    #[must_use]
    pub fn query(&self, query: CoreQuery) -> QueryResult {
        match query {
            CoreQuery::EmulatorState => QueryResult::EmulatorState(EmulatorState {
                paused: self.paused,
                speed_permille: self.speed_permille,
                controller_bits: self.ports.controllers[0].bits,
                controller2_bits: self.ports.controllers[1].bits,
            }),
            CoreQuery::Registers => QueryResult::Registers(Box::new(self.cpu.snapshot())),
            CoreQuery::Memory(addr) => QueryResult::Memory(self.read_memory(addr)),
            CoreQuery::FpsMilli => QueryResult::FpsMilli(self.fps_milli()),
            CoreQuery::PpuFrameCounter => QueryResult::PpuFrameCounter(self.ppu.frame_counter()),
        }
    }

    fn step_single_instruction(&mut self) -> Result<u64, CoreError> {
        // Sync before the CPU step so reads during execution see current PPU/APU state.
        // No post-step sync is needed: the next call's pre-step sync will refresh the
        // image before the CPU runs again, and nothing reads the image between iterations.
        self.sync_ppu_register_image();
        let (trace, cpu_cycles) = self
            .cpu
            .step_with_trace_and_cycles()
            .map_err(CoreError::CpuStepFailed)?;
        // When trace is disabled, `trace` is an empty String (no heap alloc) and
        // the bus_trace Vec is empty — skip the Option wrap and swap entirely.
        if trace.is_empty() {
            self.last_cpu_trace = None;
        } else {
            self.last_cpu_trace = Some(trace);
            self.last_cpu_bus_trace.clear();
            self.cpu.swap_bus_trace(&mut self.last_cpu_bus_trace);
        }

        let mut writes = core::mem::take(&mut self.scratch_writes);
        writes.clear();
        self.cpu.swap_writes(&mut writes);

        let mut mmio_reads = core::mem::take(&mut self.scratch_mmio_reads);
        mmio_reads.clear();
        self.cpu.swap_mmio_reads(&mut mmio_reads);

        self.replay_cpu_bus_side_effects(cpu_cycles, &writes, &mmio_reads);
        self.scratch_writes = writes;
        self.scratch_mmio_reads = mmio_reads;

        let cpu_cycles = u64::from(cpu_cycles);

        if let Some(page) = self.pending_oam_dma_page.take() {
            self.run_oam_dma(page);
        }
        if self.ppu.take_nmi_pending() {
            self.cpu.service_nmi();
            self.advance_hardware_cycles(7);
        } else if (self.apu.irq_pending() || self.mapper_irq_pending()) && self.cpu.service_irq() {
            self.advance_hardware_cycles(7);
        }
        Ok(cpu_cycles)
    }

    fn step_until_next_scanline(&mut self) -> Result<(), CoreError> {
        let start_scanline = self.ppu.scanline();
        while self.ppu.scanline() == start_scanline {
            let _ = self.step_single_instruction()?;
        }
        Ok(())
    }

    fn step_until_next_frame(&mut self) -> Result<(), CoreError> {
        let start_frame = self.ppu.frame_counter();
        while self.ppu.frame_counter() == start_frame {
            let _ = self.step_single_instruction()?;
        }
        Ok(())
    }

    fn reset_runtime(&mut self) {
        self.paused = false;
        self.ports.controllers = [ControllerState::default(); 2];
        self.ports.controller_strobe = false;
        self.scheduler.reset();
        self.ppu.reset();
        self.apu.reset();
        self.cpu.reset(self.reset_pc);
        self.pending_oam_dma_page = None;
        self.sync_ppu_register_image();
        self.last_cpu_trace = None;
        self.last_cpu_bus_trace.clear();
        self.scratch_writes.clear();
    }

    fn build_mapper(
        &self,
        mapper_id: u8,
        prg_rom: &[u8],
        chr_rom: &[u8],
        mirroring: NametableMirroring,
    ) -> Result<LoadedMapper, CoreError> {
        match mapper_id {
            0 => self.build_nrom(prg_rom),
            1 => self.build_mmc1(prg_rom),
            2 => self.build_uxrom(prg_rom),
            3 => self.build_cnrom(prg_rom, chr_rom),
            4 => self.build_mmc3(prg_rom, chr_rom, mirroring),
            7 => self.build_axrom(prg_rom),
            9 => self.build_mmc2(prg_rom, chr_rom),
            10 => self.build_mmc4(prg_rom, chr_rom),
            11 => self.build_color_dreams(prg_rom, chr_rom),
            66 => self.build_gxrom(prg_rom, chr_rom),
            69 => self.build_fme7(prg_rom, chr_rom),
            71 => self.build_camerica(prg_rom),
            206 => self.build_namco108(prg_rom, chr_rom),
            _ => Err(CoreError::RomLoadFailed(RomError::UnsupportedMapper(
                mapper_id,
            ))),
        }
    }

    fn build_nrom(&self, prg_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        match prg_rom.len() {
            PRG_16K_BYTES | PRG_32K_BYTES => {
                Ok(LoadedMapper::Nrom(Nrom::from_prg_rom(prg_rom.to_vec())))
            }
            other => Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                other,
            ))),
        }
    }

    fn build_uxrom(&self, prg_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_BANK_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        Ok(LoadedMapper::Uxrom(Uxrom::from_prg_rom(prg_rom.to_vec())))
    }

    fn build_mmc1(&self, prg_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_BANK_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        Ok(LoadedMapper::Mmc1(Mmc1::from_prg_rom(prg_rom.to_vec(), 1)))
    }

    fn build_axrom(&self, prg_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_32K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        Ok(LoadedMapper::Axrom(Axrom::from_prg_rom(prg_rom.to_vec())))
    }

    fn build_cnrom(&self, prg_rom: &[u8], chr_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        match prg_rom.len() {
            PRG_16K_BYTES | PRG_32K_BYTES => {}
            other => {
                return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                    other,
                )));
            }
        }
        if !chr_rom.is_empty() && !chr_rom.len().is_multiple_of(CHR_8K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                chr_rom.len(),
            )));
        }
        Ok(LoadedMapper::Cnrom(Cnrom::from_prg_chr(
            prg_rom.to_vec(),
            chr_rom.to_vec(),
        )))
    }

    fn build_gxrom(&self, prg_rom: &[u8], chr_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_32K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        if !chr_rom.is_empty() && !chr_rom.len().is_multiple_of(CHR_8K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                chr_rom.len(),
            )));
        }
        Ok(LoadedMapper::Gxrom(Gxrom::from_prg_chr(
            prg_rom.to_vec(),
            chr_rom.to_vec(),
        )))
    }

    fn build_color_dreams(
        &self,
        prg_rom: &[u8],
        chr_rom: &[u8],
    ) -> Result<LoadedMapper, CoreError> {
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_32K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        if !chr_rom.is_empty() && !chr_rom.len().is_multiple_of(CHR_8K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                chr_rom.len(),
            )));
        }
        Ok(LoadedMapper::ColorDreams(ColorDreams::from_prg_chr(
            prg_rom.to_vec(),
            chr_rom.to_vec(),
        )))
    }

    fn build_camerica(&self, prg_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_BANK_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        Ok(LoadedMapper::Camerica(Camerica::from_prg_rom(
            prg_rom.to_vec(),
        )))
    }

    fn build_namco108(&self, prg_rom: &[u8], chr_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_8K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        if chr_rom.is_empty() || !chr_rom.len().is_multiple_of(CHR_8K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                chr_rom.len(),
            )));
        }
        Ok(LoadedMapper::Namco108(Namco108::from_prg_chr(
            prg_rom.to_vec(),
            chr_rom.to_vec(),
        )))
    }

    fn build_fme7(&self, prg_rom: &[u8], chr_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        if prg_rom.len() < PRG_16K_BYTES || !prg_rom.len().is_multiple_of(PRG_8K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        if !chr_rom.is_empty() && !chr_rom.len().is_multiple_of(CHR_8K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                chr_rom.len(),
            )));
        }
        Ok(LoadedMapper::Fme7(Fme7::from_prg_chr(
            prg_rom.to_vec(),
            chr_rom.to_vec(),
        )))
    }

    fn build_mmc2(&self, prg_rom: &[u8], chr_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        // MMC2 (Punch-Out!!): switchable 8KB PRG bank + fixed last three banks.
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_8K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        // MMC2 titles ship CHR-ROM in 4KB banks (at least 8KB).
        if chr_rom.len() < CHR_8K_BYTES || !chr_rom.len().is_multiple_of(CHR_4K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                chr_rom.len(),
            )));
        }
        Ok(LoadedMapper::Mmc2(Mmc2::from_prg_chr(
            prg_rom.to_vec(),
            chr_rom.to_vec(),
        )))
    }

    fn build_mmc4(&self, prg_rom: &[u8], chr_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        // MMC4 (Fire Emblem): switchable 16KB PRG bank + fixed last 16KB bank.
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_16K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        // MMC4 titles ship CHR-ROM in 4KB banks (at least 8KB).
        if chr_rom.len() < CHR_8K_BYTES || !chr_rom.len().is_multiple_of(CHR_4K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                chr_rom.len(),
            )));
        }
        Ok(LoadedMapper::Mmc4(Mmc4::from_prg_chr(
            prg_rom.to_vec(),
            chr_rom.to_vec(),
        )))
    }

    fn build_mmc3(
        &self,
        prg_rom: &[u8],
        chr_rom: &[u8],
        mirroring: NametableMirroring,
    ) -> Result<LoadedMapper, CoreError> {
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_8K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        if !chr_rom.is_empty() && !chr_rom.len().is_multiple_of(CHR_8K_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                chr_rom.len(),
            )));
        }
        Ok(LoadedMapper::Mmc3(Mmc3::from_prg_chr(
            prg_rom.to_vec(),
            chr_rom.to_vec(),
            mirroring,
        )))
    }

    fn sync_mapper_prg_window(&mut self) {
        if let Some(mapper) = self.mapper.as_ref() {
            for addr in 0x8000..=0xFFFF {
                let value = self.apply_cheat_codes(addr, mapper.read_prg(addr));
                self.cpu.write_byte(addr, value);
            }
        }
        // Materialize the $6000-$7FFF PRG-RAM window for mappers that expose
        // work RAM. Mappers without WRAM return `None` for every address, so
        // the CPU flat image is left exactly as before.
        self.sync_mapper_prg_ram_window();
    }

    fn sync_mapper_prg_ram_window(&mut self) {
        let Some(mapper) = self.mapper.as_ref() else {
            return;
        };
        for addr in 0x6000..=0x7FFF {
            if let Some(raw) = mapper.read_prg_ram(addr) {
                let value = self.apply_cheat_codes(addr, raw);
                self.cpu.write_byte(addr, value);
            }
        }
    }

    fn sync_mapper_chr_window(&mut self) {
        // Gate per-pattern-fetch CHR recording to mappers that use the tile
        // latch (MMC2/MMC4). This is the single source of truth and is re-run on
        // every load and PRG-write remap, so the flag can never go stale.
        let wants_fetch = self
            .mapper
            .as_ref()
            .is_some_and(LoadedMapper::wants_chr_fetch_notify);
        self.ppu.set_chr_fetch_recording(wants_fetch);

        let Some(mapper) = self.mapper.as_ref() else {
            return;
        };
        let Some((chr_window, writable)) = mapper.chr_window() else {
            return;
        };
        self.ppu.set_chr_window(&chr_window, writable);
    }

    fn sync_mapper_mirroring(&mut self) {
        let Some(mapper) = self.mapper.as_ref() else {
            return;
        };
        let Some(mirroring) = mapper.mirroring_override() else {
            return;
        };
        self.ppu.set_mirroring(mirroring);
    }

    fn replay_cpu_bus_side_effects(
        &mut self,
        cpu_cycles: u8,
        writes: &[CpuWrite],
        mmio_reads: &[CpuMmioRead],
    ) {
        if writes.is_empty() && mmio_reads.is_empty() {
            self.advance_hardware_cycles(u64::from(cpu_cycles));
            return;
        }

        let mut write_index = 0usize;
        let mut read_index = 0usize;

        for cycle in 1..=cpu_cycles {
            while read_index < mmio_reads.len() && mmio_reads[read_index].bus_cycle == cycle {
                self.apply_cpu_read_side_effect(mmio_reads[read_index].addr);
                read_index += 1;
            }

            while write_index < writes.len() && writes[write_index].bus_cycle == cycle {
                let write = writes[write_index];
                self.apply_cpu_write_side_effect(write.addr, write.value);
                write_index += 1;
            }

            self.advance_hardware_cycles(1);
        }

        while read_index < mmio_reads.len() {
            self.apply_cpu_read_side_effect(mmio_reads[read_index].addr);
            read_index += 1;
        }
        while write_index < writes.len() {
            let write = writes[write_index];
            self.apply_cpu_write_side_effect(write.addr, write.value);
            write_index += 1;
        }
    }

    fn apply_cpu_write_side_effect(&mut self, addr: u16, value: u8) {
        let remap_needed = if addr >= 0x8000 {
            if let Some(mapper) = self.mapper.as_mut() {
                // Persist CHR-RAM writes made through PPUDATA before bank remapping.
                let chr_window = self.ppu.chr_window_snapshot();
                mapper.sync_chr_ram_from_ppu_window(&chr_window);
                mapper.write_prg(addr, value);
                true
            } else {
                false
            }
        } else {
            false
        };

        // Route $6000-$7FFF writes to cartridge PRG-RAM. Mappers without work
        // RAM ignore the write and re-materialize nothing, so the flat image
        // keeps the written byte exactly as before.
        let prg_ram_write_needed = if (0x6000..=0x7FFF).contains(&addr) {
            if let Some(mapper) = self.mapper.as_mut() {
                mapper.write_prg_ram(addr, value);
                true
            } else {
                false
            }
        } else {
            false
        };

        let ppu_changed = if (0x2000..=0x3FFF).contains(&addr) {
            self.ppu
                .write_register(normalize_ppu_register_addr(addr), value);
            true
        } else {
            false
        };

        if (0x4000..=0x4017).contains(&addr) {
            if addr == 0x4014 {
                self.pending_oam_dma_page = Some(value);
            } else if addr == 0x4016 {
                self.ports.write_controller_strobe(value);
            } else {
                self.apu.write_register(addr, value);
            }
        }

        if remap_needed {
            self.sync_mapper_prg_window();
            self.sync_mapper_mirroring();
            self.sync_mapper_chr_window();
        }
        if prg_ram_write_needed {
            // Re-materialize the $6000-$7FFF window so a subsequent CPU read
            // observes the mapper's stored byte (respecting write protection /
            // chip-enable), rather than the raw flat write.
            self.sync_mapper_prg_ram_window();
        }
        if ppu_changed {
            self.sync_ppu_register_image();
        }
    }

    fn mapper_hash_component(&self) -> u64 {
        match self.mapper.as_ref() {
            None => 0,
            Some(LoadedMapper::Nrom(_)) => 0x10,
            Some(LoadedMapper::Uxrom(mapper)) => 0x20 ^ mapper.selected_bank() as u64,
            Some(LoadedMapper::Mmc1(mapper)) => 0x30 ^ mapper.selected_prg_bank() as u64,
            Some(LoadedMapper::Cnrom(mapper)) => 0x35 ^ mapper.selected_chr_bank() as u64,
            Some(LoadedMapper::Axrom(mapper)) => {
                0x37 ^ u64::from(mapper.selected_bank())
                    ^ (u64::from(mapper.selected_nametable_bank()) << 8)
            }
            Some(LoadedMapper::Gxrom(mapper)) => {
                0x38 ^ u64::from(mapper.selected_prg_bank())
                    ^ (u64::from(mapper.selected_chr_bank()) << 8)
            }
            Some(LoadedMapper::Mmc3(mapper)) => {
                0x40 ^ (u64::from(mapper.read_prg(0x8000)) << 8)
                    ^ (u64::from(mapper.read_prg(0xA000)) << 16)
            }
            Some(LoadedMapper::ColorDreams(mapper)) => {
                0x50 ^ u64::from(mapper.selected_prg_bank())
                    ^ (u64::from(mapper.selected_chr_bank()) << 8)
            }
            Some(LoadedMapper::Camerica(mapper)) => 0x51 ^ u64::from(mapper.selected_bank()),
            Some(LoadedMapper::Namco108(mapper)) => {
                0x52 ^ (u64::from(mapper.read_prg(0x8000)) << 8)
                    ^ (u64::from(mapper.read_prg(0xA000)) << 16)
            }
            Some(LoadedMapper::Fme7(mapper)) => {
                0x53 ^ (u64::from(mapper.read_prg(0x8000)) << 8)
                    ^ (u64::from(mapper.read_prg(0xA000)) << 16)
                    ^ (u64::from(mapper.irq_pending()) << 32)
            }
            Some(LoadedMapper::Mmc2(mapper)) => {
                0x54 ^ (u64::from(mapper.read_prg(0x8000)) << 8)
                    ^ (u64::from(mapper.chr_window()[0x0000]) << 16)
                    ^ (u64::from(mapper.chr_window()[0x1000]) << 24)
            }
            Some(LoadedMapper::Mmc4(mapper)) => {
                0x55 ^ (u64::from(mapper.read_prg(0x8000)) << 8)
                    ^ (u64::from(mapper.chr_window()[0x0000]) << 16)
                    ^ (u64::from(mapper.chr_window()[0x1000]) << 24)
            }
        }
    }

    fn cheat_code_hash_component(&self) -> u64 {
        let mut hash = 0_u64;
        for (index, code) in self.cheat_codes.iter().enumerate() {
            let compare = code.compare().map_or(0x1FF_u64, u64::from);
            let component = u64::from(code.address())
                ^ (u64::from(code.value()) << 16)
                ^ (compare << 24)
                ^ ((index as u64) << 40);
            hash ^= component.rotate_left(((index % 63) + 1) as u32);
        }
        hash
    }

    fn sync_ppu_register_image(&mut self) {
        self.cpu.write_byte(0x2000, self.ppu.ctrl());
        self.cpu.write_byte(0x2001, self.ppu.mask());
        self.cpu.write_byte(0x2002, self.ppu.status());
        self.cpu
            .write_byte(0x2004, self.ppu.peek_oam_data_for_cpu_read());
        self.cpu
            .write_byte(0x2007, self.ppu.peek_data_for_cpu_read());
        self.cpu.write_byte(0x4015, self.apu.peek_status());
        self.cpu
            .write_byte(0x4016, self.ports.controller_port_sample(Player::One));
        self.cpu
            .write_byte(0x4017, self.ports.controller_port_sample(Player::Two));
    }

    fn apply_cpu_read_side_effect(&mut self, addr: u16) {
        match addr {
            0x2002 => self.ppu.on_status_read(),
            0x2007 => {
                let _ = self.ppu.consume_data_read();
            }
            0x4015 => {
                let _ = self.apu.read_status();
            }
            0x4016 => self.ports.consume_controller_read(Player::One),
            0x4017 => self.ports.consume_controller_read(Player::Two),
            _ => {}
        }
    }

    fn apply_cheat_codes(&self, addr: u16, original: u8) -> u8 {
        let mut patched = original;
        for code in &self.cheat_codes {
            if code.applies_to(addr, original) {
                patched = code.value();
            }
        }
        patched
    }

    fn mapper_irq_pending(&self) -> bool {
        self.mapper.as_ref().is_some_and(LoadedMapper::irq_pending)
    }

    fn advance_hardware_cycles(&mut self, cycles: u64) {
        // Batch-advance scheduler accounting counters once for the whole burst.
        // The counters are not observed mid-loop, so this is equivalent to N
        // per-cycle calls but avoids 5 wrapping_add operations per CPU cycle.
        // One CPU cycle = 1 APU cycle = 3 PPU cycles.
        self.scheduler.advance_by(cycles);

        for _ in 0..cycles {
            let dmc_request = self.apu.step_cpu_cycle(self.paused);
            for _ in 0..3 {
                self.ppu.step_dot();
                if let Some(mapper) = self.mapper.as_mut() {
                    mapper.on_ppu_dot(
                        self.ppu.scanline(),
                        self.ppu.dot(),
                        self.ppu.rendering_enabled_for_mapper_irq(),
                        self.ppu.ctrl(),
                    );
                }
                self.pump_mapper_chr_fetches();
            }
            if let Some(request) = dmc_request {
                self.apply_dmc_dma_request(request);
            }
        }
    }

    /// Drains the PPU's per-dot pattern-fetch log into the mapper's CHR tile
    /// latch (MMC2/MMC4). When a latch flips, the mapper's CHR window changed,
    /// so it is re-synced to the PPU (deferred to the next prefetch boundary,
    /// exactly like an MMC3 mid-frame CHR split). For every non-latching mapper
    /// the PPU records nothing, so `take_chr_fetches` returns 0 and this is a
    /// single cheap early return — zero behavior change.
    fn pump_mapper_chr_fetches(&mut self) {
        let mut buf = [0_u16; CHR_FETCH_BUF_LEN];
        let count = self.ppu.take_chr_fetches(&mut buf);
        if count == 0 {
            return;
        }
        let mut changed = false;
        if let Some(mapper) = self.mapper.as_mut() {
            for &addr in &buf[..count] {
                if mapper.notify_ppu_chr_fetch(addr) {
                    changed = true;
                }
            }
        }
        if changed {
            self.sync_mapper_chr_window();
        }
    }

    fn apply_dmc_dma_request(&mut self, request: DmcDmaRequest) {
        let sample = self.cpu.read_byte(request.addr);
        self.apu.load_dmc_sample(sample);
        for _ in 0..request.stall_cycles {
            self.scheduler.step_cpu_cycle();
            self.scheduler.step_apu_cycle();
            let dmc_request = self.apu.step_cpu_cycle(self.paused);
            for _ in 0..3 {
                self.scheduler.step_ppu_cycle();
                self.ppu.step_dot();
                if let Some(mapper) = self.mapper.as_mut() {
                    mapper.on_ppu_dot(
                        self.ppu.scanline(),
                        self.ppu.dot(),
                        self.ppu.rendering_enabled_for_mapper_irq(),
                        self.ppu.ctrl(),
                    );
                }
                self.pump_mapper_chr_fetches();
            }
            if let Some(chained) = dmc_request {
                let byte = self.cpu.read_byte(chained.addr);
                self.apu.load_dmc_sample(byte);
            }
        }
    }

    fn run_oam_dma(&mut self, page: u8) {
        let mut bytes = [0_u8; 256];
        let base = u16::from(page) << 8;
        for (offset, slot) in bytes.iter_mut().enumerate() {
            *slot = self.cpu.read_byte(base.wrapping_add(offset as u16));
        }
        self.ppu.dma_oam(&bytes);

        let stall_cycles = if self.scheduler.cpu_cycles().is_multiple_of(2) {
            514
        } else {
            513
        };
        self.advance_hardware_cycles(stall_cycles);
    }
}

#[must_use]
fn normalize_ppu_register_addr(addr: u16) -> u16 {
    0x2000 + ((addr - 0x2000) % 8)
}

impl Default for NesCore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_chr_window_for_mmc3() {
        let mmc3 = Mmc3::from_prg_chr(
            vec![0; 32 * 1024],
            vec![0; 8 * 1024],
            NametableMirroring::Vertical,
        );
        let mapper = LoadedMapper::Mmc3(mmc3);
        assert!(mapper.chr_window().is_some());
    }

    #[test]
    fn should_return_mirroring_override_for_axrom() {
        let axrom = Axrom::from_prg_rom(vec![0; 32 * 1024]);
        let mapper = LoadedMapper::Axrom(axrom);
        assert_eq!(
            mapper.mirroring_override(),
            Some(NametableMirroring::OneScreenLower)
        );
    }

    #[test]
    fn should_return_mirroring_override_for_mmc3() {
        let mmc3 = Mmc3::from_prg_chr(
            vec![0; 32 * 1024],
            vec![0; 8 * 1024],
            NametableMirroring::Vertical,
        );
        let mapper = LoadedMapper::Mmc3(mmc3);
        assert_eq!(
            mapper.mirroring_override(),
            Some(NametableMirroring::Vertical)
        );
    }

    #[test]
    fn should_return_irq_pending_for_mmc3() {
        let mut mmc3 = Mmc3::from_prg_chr(
            vec![0; 32 * 1024],
            vec![0; 8 * 1024],
            NametableMirroring::Vertical,
        );
        // Force an IRQ
        mmc3.write_prg(0xC000, 1); // IRQ reload value
        mmc3.write_prg(0xC001, 0); // IRQ reload clear
        mmc3.write_prg(0xE001, 0); // IRQ enable
        // Drive two full visible scanlines of A12 edges (BG=$0000, sprites=$1000)
        // to reload then decrement the counter to zero.
        for scanline in 0..2 {
            for dot in 0..=340 {
                mmc3.on_ppu_dot(scanline, dot, true, 0x08);
            }
        }

        let mapper = LoadedMapper::Mmc3(mmc3);
        assert!(mapper.irq_pending());
    }

    #[test]
    fn should_call_on_ppu_dot_for_mmc3() {
        let mut mmc3 = Mmc3::from_prg_chr(
            vec![0; 32 * 1024],
            vec![0; 8 * 1024],
            NametableMirroring::Vertical,
        );

        mmc3.write_prg(0xC000, 1);
        mmc3.write_prg(0xC001, 0);
        mmc3.write_prg(0xE001, 0);

        let mut mapper = LoadedMapper::Mmc3(mmc3);

        for scanline in 0..2 {
            for dot in 0..=340 {
                mapper.on_ppu_dot(scanline, dot, true, 0x08);
            }
        }

        assert!(mapper.irq_pending());
    }

    #[test]
    fn should_sync_chr_ram_for_cnrom() {
        let cnrom = Cnrom::from_prg_chr(vec![0; 32 * 1024], vec![]);
        let mut mapper = LoadedMapper::Cnrom(cnrom);
        let window = [1; 8192];
        mapper.sync_chr_ram_from_ppu_window(&window);

        // Since Cnrom has writable CHR-RAM here, the window should have synced to its RAM.
        // There is no easy getter for CHR-RAM, but we can call chr_window.
        let (chr_window, writable) = mapper.chr_window().unwrap();
        assert!(writable);
        assert_eq!(chr_window[0], 1);
    }

    #[test]
    fn should_sync_chr_ram_for_gxrom() {
        let gxrom = Gxrom::from_prg_chr(vec![0; 32 * 1024], vec![]);
        let mut mapper = LoadedMapper::Gxrom(gxrom);
        let window = [2; 8192];
        mapper.sync_chr_ram_from_ppu_window(&window);

        let (chr_window, writable) = mapper.chr_window().unwrap();
        assert!(writable);
        assert_eq!(chr_window[0], 2);
    }

    #[test]
    fn should_return_chr_writable_for_cnrom() {
        let cnrom = Cnrom::from_prg_chr(vec![0; 32 * 1024], vec![]);
        let mapper = LoadedMapper::Cnrom(cnrom);
        assert!(mapper.chr_writable());
    }

    #[test]
    fn should_return_chr_writable_for_gxrom() {
        let gxrom = Gxrom::from_prg_chr(vec![0; 32 * 1024], vec![]);
        let mapper = LoadedMapper::Gxrom(gxrom);
        assert!(mapper.chr_writable());
    }

    #[test]
    fn should_return_chr_writable_for_mmc3() {
        let mmc3 = Mmc3::from_prg_chr(vec![0; 32 * 1024], vec![], NametableMirroring::Vertical);
        let mapper = LoadedMapper::Mmc3(mmc3);
        assert!(mapper.chr_writable());
    }

    #[test]
    fn should_return_false_for_chr_writable_unsupported_mappers() {
        let nrom = Nrom::from_prg_rom(vec![0; 32 * 1024]);
        let mapper = LoadedMapper::Nrom(nrom);
        assert!(!mapper.chr_writable());
    }

    #[test]
    fn should_sync_chr_ram_from_ppu_window_for_mmc3() {
        let mmc3 = Mmc3::from_prg_chr(vec![0; 32 * 1024], vec![], NametableMirroring::Vertical);
        let mut mapper = LoadedMapper::Mmc3(mmc3);
        let window = [3; 8192];
        mapper.sync_chr_ram_from_ppu_window(&window);

        let (chr_window, writable) = mapper.chr_window().unwrap();
        assert!(writable);
        assert_eq!(chr_window[0], 3);
    }

    #[test]
    fn should_ignore_sync_chr_ram_for_unsupported_mappers() {
        let nrom = Nrom::from_prg_rom(vec![0; 32 * 1024]);
        let mut mapper = LoadedMapper::Nrom(nrom);
        let window = [4; 8192];
        // This should not panic or change anything
        mapper.sync_chr_ram_from_ppu_window(&window);
    }

    #[test]
    fn should_ignore_on_ppu_dot_for_unsupported_mappers() {
        let nrom = Nrom::from_prg_rom(vec![0; 32 * 1024]);
        let mut mapper = LoadedMapper::Nrom(nrom);
        // This should not panic
        mapper.on_ppu_dot(0, 0, true, 0x08);
    }

    #[test]
    fn should_return_false_for_irq_pending_unsupported_mappers() {
        let nrom = Nrom::from_prg_rom(vec![0; 32 * 1024]);
        let mapper = LoadedMapper::Nrom(nrom);
        assert!(!mapper.irq_pending());
    }

    #[test]
    fn should_return_true_for_irq_pending_when_mmc3_has_irq() {
        let mut mmc3 = Mmc3::from_prg_chr(vec![0; 32 * 1024], vec![], NametableMirroring::Vertical);
        // Force an IRQ
        mmc3.write_prg(0xC000, 1);
        mmc3.write_prg(0xC001, 0);
        mmc3.write_prg(0xE001, 0);
        for scanline in 0..2 {
            for dot in 0..=340 {
                mmc3.on_ppu_dot(scanline, dot, true, 0x08);
            }
        }
        let mapper = LoadedMapper::Mmc3(mmc3);
        assert!(mapper.irq_pending());
    }

    #[test]
    fn should_return_none_for_mirroring_override_unsupported_mappers() {
        let nrom = Nrom::from_prg_rom(vec![0; 32 * 1024]);
        let mapper = LoadedMapper::Nrom(nrom);
        assert_eq!(mapper.mirroring_override(), None);
    }

    #[test]
    fn should_return_none_for_chr_window_unsupported_mappers() {
        let nrom = Nrom::from_prg_rom(vec![0; 32 * 1024]);
        let mapper = LoadedMapper::Nrom(nrom);
        assert_eq!(mapper.chr_window(), None);
    }

    #[test]
    fn should_return_chr_window_for_gxrom() {
        let gxrom = Gxrom::from_prg_chr(vec![0; 32 * 1024], vec![]);
        let mapper = LoadedMapper::Gxrom(gxrom);
        assert!(mapper.chr_window().is_some());
    }

    #[test]
    fn should_return_chr_window_for_cnrom() {
        let cnrom = Cnrom::from_prg_chr(vec![0; 32 * 1024], vec![]);
        let mapper = LoadedMapper::Cnrom(cnrom);
        assert!(mapper.chr_window().is_some());
    }

    #[test]
    fn should_return_none_mirroring_override_for_nrom() {
        let nrom = Nrom::from_prg_rom(vec![0; 32 * 1024]);
        let mapper = LoadedMapper::Nrom(nrom);
        assert_eq!(mapper.mirroring_override(), None);
    }

    #[test]
    fn set_speed_returns_error_on_zero() {
        let mut core = NesCore::new();
        let result = core.execute(Command::SetSpeed(0));
        assert_eq!(result, Err(CoreError::InvalidSpeed(0)));
    }

    #[test]
    fn command_release_button_clears_controller_bit() {
        let mut core = NesCore::new();
        core.execute(Command::PressButton(Button::A)).unwrap();
        assert_ne!(core.controller_bits() & Button::A.bit_mask(), 0);
        core.execute(Command::ReleaseButton(Button::A)).unwrap();
        assert_eq!(core.controller_bits() & Button::A.bit_mask(), 0);

        core.execute(Command::PressButton2(Button::B)).unwrap();
        assert_ne!(core.controller2_bits() & Button::B.bit_mask(), 0);
        core.execute(Command::ReleaseButton2(Button::B)).unwrap();
        assert_eq!(core.controller2_bits() & Button::B.bit_mask(), 0);
    }

    #[test]
    fn command_power_cycle_resets_speed_to_default() {
        let mut core = NesCore::new();
        core.execute(Command::SetSpeed(2000)).unwrap();
        assert_eq!(core.speed_permille(), 2000);
        core.execute(Command::PowerCycle).unwrap();
        assert_eq!(core.speed_permille(), DEFAULT_SPEED_PERMILLE);
    }

    #[test]
    fn should_return_correct_bit_mask_for_all_buttons() {
        assert_eq!(Button::A.bit_mask(), 0b0000_0001);
        assert_eq!(Button::B.bit_mask(), 0b0000_0010);
        assert_eq!(Button::Select.bit_mask(), 0b0000_0100);
        assert_eq!(Button::Start.bit_mask(), 0b0000_1000);
        assert_eq!(Button::Up.bit_mask(), 0b0001_0000);
        assert_eq!(Button::Down.bit_mask(), 0b0010_0000);
        assert_eq!(Button::Left.bit_mask(), 0b0100_0000);
        assert_eq!(Button::Right.bit_mask(), 0b1000_0000);
    }

    #[test]
    fn should_return_correct_index_for_players() {
        assert_eq!(Player::One.index(), 0);
        assert_eq!(Player::Two.index(), 1);
    }

    #[test]
    fn core_query_returns_expected_variants() {
        let mut core = NesCore::new();
        // FpsMilli
        core.execute(Command::SetSpeed(2000)).unwrap();
        if let QueryResult::FpsMilli(fps) = core.query(CoreQuery::FpsMilli) {
            assert_eq!(fps, 120_000);
        } else {
            panic!("Expected FpsMilli");
        }

        // PpuFrameCounter
        if let QueryResult::PpuFrameCounter(frames) = core.query(CoreQuery::PpuFrameCounter) {
            assert_eq!(frames, 0);
        } else {
            panic!("Expected PpuFrameCounter");
        }

        // Registers
        if let QueryResult::Registers(regs) = core.query(CoreQuery::Registers) {
            assert_eq!(regs.pc, DEFAULT_START_PC);
        } else {
            panic!("Expected Registers");
        }

        // Memory
        if let QueryResult::Memory(val) = core.query(CoreQuery::Memory(0x0000)) {
            assert_eq!(val, 0);
        } else {
            panic!("Expected Memory");
        }
    }

    #[test]
    fn test_core_snapshot_mapper_delta() {
        let core1 = NesCore::new();
        let snap1 = core1.save_state();
        let core2 = NesCore::new();
        let mut snap2 = core2.save_state();

        let delta = snap1.mapper_delta(&snap2);
        assert!(delta.is_none());

        let delta = MapperDelta {
            kind: MapperDeltaKind::Replace(None),
        };
        snap2.apply_mapper_delta(&delta);
        assert!(snap2.mapper.is_none());
    }

    #[test]
    fn test_core_error_display() {
        assert_eq!(
            format!("{}", CoreError::UnsupportedCommand),
            "unsupported command"
        );
        assert_eq!(
            format!("{}", CoreError::InvalidSpeed(123)),
            "invalid speed multiplier: 123"
        );
        assert_eq!(
            format!(
                "{}",
                CoreError::RomLoadFailed(crate::rom::RomError::InvalidMagic)
            ),
            "rom load failed: invalid iNES magic header"
        );
        assert_eq!(
            format!(
                "{}",
                CoreError::CpuStepFailed(crate::cpu::CpuError::UnknownOpcode(0x12))
            ),
            "cpu step failed: unknown opcode 0x12"
        );
    }

    #[test]
    fn test_nescore_default() {
        let core = NesCore::default();
        assert!(core.mapper.is_none());
    }

    #[test]
    fn test_core_query_fps_and_frame() {
        let core = NesCore::new();
        let fps = core.query(CoreQuery::FpsMilli);
        assert!(matches!(fps, QueryResult::FpsMilli(_)));

        let p_frame = core.query(CoreQuery::PpuFrameCounter);
        assert!(matches!(p_frame, QueryResult::PpuFrameCounter(_)));
    }

    #[test]
    fn test_mapper_hash_components() {
        use crate::mapper::Axrom;
        use crate::mapper::Cnrom;
        use crate::mapper::Gxrom;
        use crate::mapper::Mmc1;
        use crate::mapper::Mmc3;
        use crate::mapper::Nrom;
        use crate::mapper::Uxrom;
        use crate::rom::NametableMirroring;

        let mut core = NesCore::new();

        let nrom = Nrom::from_prg_rom(vec![0; 32 * 1024]);
        core.mapper = Some(LoadedMapper::Nrom(nrom));
        assert_eq!(core.mapper_hash_component(), 0x10);

        let uxrom = Uxrom::from_prg_rom(vec![0; 64 * 1024]);
        core.mapper = Some(LoadedMapper::Uxrom(uxrom));
        assert_eq!(core.mapper_hash_component(), 0x20);

        let mmc1 = Mmc1::from_prg_rom(vec![0; 64 * 1024], 0);
        core.mapper = Some(LoadedMapper::Mmc1(mmc1));
        assert_eq!(core.mapper_hash_component(), 0x30);

        let cnrom = Cnrom::from_prg_chr(vec![0; 32 * 1024], vec![]);
        core.mapper = Some(LoadedMapper::Cnrom(cnrom));
        assert_eq!(core.mapper_hash_component(), 0x35);

        let axrom = Axrom::from_prg_rom(vec![0; 64 * 1024]);
        core.mapper = Some(LoadedMapper::Axrom(axrom));
        assert_eq!(core.mapper_hash_component(), 0x37);

        let gxrom = Gxrom::from_prg_chr(vec![0; 32 * 1024], vec![]);
        core.mapper = Some(LoadedMapper::Gxrom(gxrom));
        assert_eq!(core.mapper_hash_component(), 0x38);

        let mmc3 = Mmc3::from_prg_chr(vec![0; 64 * 1024], vec![], NametableMirroring::Vertical);
        core.mapper = Some(LoadedMapper::Mmc3(mmc3));
        let mmc3_hash = core.mapper_hash_component();
        let bank_8000 = core.mapper.as_ref().unwrap().read_prg(0x8000);
        let bank_a000 = core.mapper.as_ref().unwrap().read_prg(0xA000);
        assert_eq!(
            mmc3_hash,
            0x40 ^ ((bank_8000 as u64) << 8) ^ ((bank_a000 as u64) << 16)
        );
    }

    #[test]
    fn test_cheat_code_hash_component() {
        use std::str::FromStr;
        let mut core = NesCore::new();
        let code = crate::cheat_codes::CheatCode::from_str("SXTPOU").unwrap();
        core.cheat_codes.push(code.clone());
        let hash = core.cheat_code_hash_component();
        let compare = 0x1FF_u64;
        let expected_comp =
            u64::from(code.address()) ^ (u64::from(code.value()) << 16) ^ (compare << 24);
        let expected_hash = expected_comp.rotate_left(1);
        assert_eq!(hash, expected_hash);
    }
}

#[cfg(test)]
mod tests_rom_loader_internal {
    use super::*;
    use crate::rom::NametableMirroring;

    #[test]
    fn test_build_mapper_invalid_sizes() {
        let core = NesCore::new();
        assert!(core.build_gxrom(&[0; 10], &[0; 10]).is_err());
        assert!(
            core.build_mmc3(&[0; 10], &[0; 10], NametableMirroring::Vertical)
                .is_err()
        );
    }

    #[test]
    fn test_build_mapper_unsupported_prg_layouts() {
        let core = NesCore::new();

        // NROM
        let err = core
            .build_mapper(0, &[0; 48 * 1024], &[], NametableMirroring::Horizontal)
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(49152))
        ));

        // MMC1
        let err = core
            .build_mapper(1, &[0; 16 * 1024], &[], NametableMirroring::Horizontal)
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(16384))
        ));

        // UXROM
        let err = core
            .build_mapper(2, &[0; 16 * 1024], &[], NametableMirroring::Horizontal)
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(16384))
        ));

        // CNROM
        let err = core
            .build_mapper(3, &[0; 48 * 1024], &[], NametableMirroring::Horizontal)
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(49152))
        ));

        let err = core
            .build_mapper(
                3,
                &[0; 16 * 1024],
                &[0; 10 * 1024],
                NametableMirroring::Horizontal,
            )
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(10240))
        ));

        // MMC3
        let err = core
            .build_mapper(4, &[0; 16 * 1024], &[], NametableMirroring::Horizontal)
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(16384))
        ));

        let err = core
            .build_mapper(
                4,
                &[0; 32 * 1024],
                &[0; 10 * 1024],
                NametableMirroring::Horizontal,
            )
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(10240))
        ));

        // AXROM
        let err = core
            .build_mapper(7, &[0; 16 * 1024], &[], NametableMirroring::Horizontal)
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(16384))
        ));

        // GXROM
        let err = core
            .build_mapper(66, &[0; 16 * 1024], &[], NametableMirroring::Horizontal)
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(16384))
        ));

        let err = core
            .build_mapper(
                66,
                &[0; 32 * 1024],
                &[0; 10 * 1024],
                NametableMirroring::Horizontal,
            )
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(10240))
        ));

        // Unsupported Mapper
        let err = core
            .build_mapper(99, &[0; 16 * 1024], &[], NametableMirroring::Horizontal)
            .unwrap_err();
        assert!(matches!(
            err,
            CoreError::RomLoadFailed(RomError::UnsupportedMapper(99))
        ));
    }

    #[test]
    fn test_build_new_mapper_size_validation_errors() {
        let core = NesCore::new();
        let mirror = NametableMirroring::Horizontal;

        // Mapper 11 (Color Dreams): PRG must be a positive multiple of 32KB;
        // CHR (when present) a multiple of 8KB.
        assert!(core.build_mapper(11, &[0; 16 * 1024], &[], mirror).is_err());
        assert!(
            core.build_mapper(11, &[0; 32 * 1024], &[0; 10 * 1024], mirror)
                .is_err()
        );

        // Mapper 71 (Camerica): PRG at least 32KB and a multiple of 16KB.
        assert!(core.build_mapper(71, &[0; 8 * 1024], &[], mirror).is_err());

        // Mapper 206 (Namco-108): PRG >= 32KB, multiple of 8KB; CHR required and
        // a multiple of 8KB.
        assert!(
            core.build_mapper(206, &[0; 16 * 1024], &[0; 8 * 1024], mirror)
                .is_err()
        );
        assert!(
            core.build_mapper(206, &[0; 32 * 1024], &[], mirror)
                .is_err()
        );
        assert!(
            core.build_mapper(206, &[0; 32 * 1024], &[0; 10 * 1024], mirror)
                .is_err()
        );

        // Mapper 69 (FME-7): PRG >= 16KB, multiple of 8KB; CHR multiple of 8KB.
        assert!(core.build_mapper(69, &[0; 8 * 1024], &[], mirror).is_err());
        assert!(
            core.build_mapper(69, &[0; 16 * 1024], &[0; 10 * 1024], mirror)
                .is_err()
        );

        // Mapper 9 (MMC2): PRG >= 32KB, multiple of 8KB; CHR >= 8KB, multiple of 4KB.
        assert!(
            core.build_mapper(9, &[0; 16 * 1024], &[0; 8 * 1024], mirror)
                .is_err()
        );
        assert!(
            core.build_mapper(9, &[0; 32 * 1024], &[0; 4 * 1024], mirror)
                .is_err()
        );

        // Mapper 10 (MMC4): PRG >= 32KB, multiple of 16KB; CHR >= 8KB, multiple of 4KB.
        assert!(
            core.build_mapper(10, &[0; 16 * 1024], &[0; 8 * 1024], mirror)
                .is_err()
        );
        assert!(
            core.build_mapper(10, &[0; 32 * 1024], &[0; 4 * 1024], mirror)
                .is_err()
        );
    }
}

#[cfg(test)]
mod tests_new_mapper_deltas {
    //! Covers the per-mapper `delta_to` / `snapshot_delta` / `apply_delta`
    //! fan-out arms added for mappers 11/71/206/69/9/10. Each round trip mutates
    //! a mapper, captures the delta, and asserts that applying it reconciles a
    //! fresh copy back to the mutated state.
    use super::*;

    fn round_trip(before: LoadedMapper, mutate: impl FnOnce(&mut LoadedMapper)) {
        let mut after = before.clone();
        mutate(&mut after);

        // Exercise the per-variant `chr_writable` fan-out arm.
        let _ = before.chr_writable();

        // The mutated mapper must produce a full-state snapshot delta for its
        // variant (exercises the `snapshot_delta` arm).
        assert!(
            after.snapshot_delta().is_some(),
            "mutated mapper should yield a snapshot delta"
        );

        // `delta_to` captures the mutation as a variant-specific delta...
        let delta = before
            .delta_to(&after)
            .expect("a mutated mapper must produce a delta");

        // ...and applying it reconciles a fresh copy of `before` into `after`.
        let mut target = before.clone();
        target.apply_delta(&delta, &[0_u8; CHR_8K_BYTES]);
        assert!(
            target.delta_to(&after).is_none(),
            "apply_delta should reconcile the mapper runtime state"
        );
    }

    #[test]
    fn colordreams_delta_round_trip() {
        let before = LoadedMapper::ColorDreams(ColorDreams::from_prg_chr(
            vec![0; 64 * 1024],
            vec![0; 8 * 1024],
        ));
        round_trip(before, |m| m.write_prg(0x8000, 0x01)); // select PRG bank 1
    }

    #[test]
    fn camerica_delta_round_trip() {
        let before = LoadedMapper::Camerica(Camerica::from_prg_rom(vec![0; 64 * 1024]));
        round_trip(before, |m| m.write_prg(0xC000, 0x02)); // select bank 2
    }

    #[test]
    fn namco108_delta_round_trip() {
        let before = LoadedMapper::Namco108(Namco108::from_prg_chr(
            vec![0; 64 * 1024],
            vec![0; 8 * 1024],
        ));
        round_trip(before, |m| {
            m.write_prg(0x8000, 6); // select register 6 (R6 -> $8000 slot)
            m.write_prg(0x8001, 3); // bank 3
        });
    }

    #[test]
    fn fme7_delta_round_trip() {
        let before = LoadedMapper::Fme7(Fme7::from_prg_chr(vec![0; 32 * 1024], vec![0; 8 * 1024]));
        round_trip(before, |m| {
            m.write_prg(0x8000, 0x09); // command: PRG reg for $8000 slot
            m.write_prg(0xA000, 3); // bank 3
        });
    }

    #[test]
    fn mmc2_delta_round_trip() {
        let before = LoadedMapper::Mmc2(Mmc2::from_prg_chr(vec![0; 32 * 1024], vec![0; 8 * 1024]));
        round_trip(before, |m| m.write_prg(0xA000, 2)); // select $8000 bank 2
    }

    #[test]
    fn mmc4_delta_round_trip() {
        let before = LoadedMapper::Mmc4(Mmc4::from_prg_chr(vec![0; 32 * 1024], vec![0; 8 * 1024]));
        round_trip(before, |m| m.write_prg(0xA000, 1)); // select $8000 bank 1
    }

    #[test]
    fn unchanged_new_mappers_produce_no_delta() {
        // The `then_some(..)` false path: an unmutated mapper yields no delta.
        let cd = LoadedMapper::ColorDreams(ColorDreams::from_prg_chr(
            vec![0; 64 * 1024],
            vec![0; 8 * 1024],
        ));
        assert!(cd.delta_to(&cd.clone()).is_none());
        let fme7 = LoadedMapper::Fme7(Fme7::from_prg_chr(vec![0; 32 * 1024], vec![0; 8 * 1024]));
        assert!(fme7.delta_to(&fme7.clone()).is_none());
    }
}

#[cfg(test)]
mod tests_api_coverage_gaps {
    use super::*;
    use crate::rom::NametableMirroring;

    #[test]
    fn test_core_snapshot_mapper_delta_coverage() {
        use crate::mapper::Mmc1;
        let mut core1 = NesCore::new();
        let mmc1 = Mmc1::from_prg_rom(vec![0; 64 * 1024], 0);
        core1.mapper = Some(LoadedMapper::Mmc1(mmc1.clone()));

        let mut core2 = NesCore::new();
        core2.mapper = Some(LoadedMapper::Mmc1(mmc1.clone()));

        let snap1 = core1.save_state();
        let mut snap2 = core2.save_state();

        // This triggers the `Some, Some` arm in CoreSnapshot::mapper_delta
        let delta = snap1.mapper_delta(&snap2);

        // Apply mapper delta
        if let Some(d) = delta {
            snap2.apply_mapper_delta(&d);
        } else {
            // Force apply a Replace delta
            let d = MapperDelta {
                kind: MapperDeltaKind::Replace(core2.mapper.clone()),
            };
            snap2.apply_mapper_delta(&d);
        }

        // Force chr_writable path by using MMC3 (which has writable CHR if RAM)
        let mut core3 = NesCore::new();
        let mmc3 = crate::mapper::Mmc3::from_prg_chr(
            vec![0; 64 * 1024],
            vec![0; 8 * 1024],
            NametableMirroring::Vertical,
        );
        core3.mapper = Some(LoadedMapper::Mmc3(mmc3.clone()));

        let snap3 = core3.save_state();
        let mut snap4 = core3.save_state();

        // Modify PPU CHR so they differ, triggering `self.ppu.chr != after.ppu.chr`
        // AND `after_mapper.chr_writable()` is true for MMC3.
        snap4.ppu.chr[0] = 99;

        let diff = snap3.mapper_delta(&snap4);

        // Apply delta if returned
        if let Some(d) = diff {
            let mut snap_apply = core3.save_state();
            snap_apply.apply_mapper_delta(&d);
        }

        // Replace from different mapper type
        let diff2 = snap1.mapper_delta(&snap4);
        assert!(diff2.is_some());
    }

    #[test]
    fn test_execute_unsupported_commands_returns_ok() {
        let mut core = NesCore::new();

        // Check `execute` match arms like `SetSpeed`
        core.execute(Command::SetSpeed(1500)).unwrap();
        assert_eq!(core.speed_permille(), 1500);
    }
}
