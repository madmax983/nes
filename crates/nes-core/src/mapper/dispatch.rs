use serde::{Deserialize, Serialize};

use crate::mapper::{
    Axrom, AxromState, Camerica, CamericaState, Cnrom, CnromState, ColorDreams, ColorDreamsState,
    Fme7, Fme7State, Gxrom, GxromState, Mmc1, Mmc1State, Mmc2, Mmc2State, Mmc3, Mmc3State, Mmc4,
    Mmc4State, Mmc5, Mmc5State, Namco108, Namco108State, Nrom, Uxrom, UxromState,
};
const CHR_8K_BYTES: usize = 8 * 1024;
use crate::rom::NametableMirroring;

/// Opaque mapper-runtime delta between two [`CoreSnapshot`]s.
///
/// This captures mutable mapper state such as bank registers, mirroring mode,
/// and IRQ counters without exposing internal mapper implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapperDelta {
    pub(crate) kind: MapperDeltaKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MapperDeltaKind {
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
    Mmc5(Mmc5State),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum LoadedMapper {
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
    Mmc5(Mmc5),
}

impl LoadedMapper {
    pub(crate) fn read_prg(&self, addr: u16) -> u8 {
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
            Self::Mmc5(mapper) => mapper.read_prg(addr),
        }
    }

    pub(crate) fn write_prg(&mut self, addr: u16, value: u8) {
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
            Self::Mmc5(mapper) => mapper.write_prg(addr, value),
        }
    }

    /// Reads a byte from cartridge PRG-RAM (`$6000..=$7FFF`), or `None` when the
    /// mapper has no work RAM at `addr`. Mappers without PRG-RAM return `None`
    /// so the CPU flat image at `$6000..=$7FFF` is left untouched.
    pub(crate) fn read_prg_ram(&self, addr: u16) -> Option<u8> {
        match self {
            Self::Mmc3(mapper) => mapper.read_prg_ram(addr),
            Self::Fme7(mapper) => mapper.read_prg_ram(addr),
            Self::Mmc4(mapper) => mapper.read_prg_ram(addr),
            Self::Mmc5(mapper) => mapper.read_prg_ram(addr),
            _ => None,
        }
    }

    /// Writes a byte to cartridge PRG-RAM (`$6000..=$7FFF`). Mappers without
    /// work RAM ignore the write.
    pub(crate) fn write_prg_ram(&mut self, addr: u16, value: u8) {
        match self {
            Self::Mmc3(mapper) => mapper.write_prg_ram(addr, value),
            Self::Fme7(mapper) => mapper.write_prg_ram(addr, value),
            Self::Mmc4(mapper) => mapper.write_prg_ram(addr, value),
            Self::Mmc5(mapper) => mapper.write_prg_ram(addr, value),
            _ => {}
        }
    }

    pub(crate) fn chr_window(&self) -> Option<([u8; CHR_8K_BYTES], bool)> {
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
            Self::Mmc5(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            _ => None,
        }
    }

    /// Separate background CHR window (MMC5 8x16-sprite mode). `None` for every
    /// mapper without an A/B CHR split, in which case backgrounds share the
    /// single [`LoadedMapper::chr_window`].
    pub(crate) fn chr_bg_window(&self) -> Option<[u8; CHR_8K_BYTES]> {
        match self {
            Self::Mmc5(mapper) => mapper.chr_bg_window(),
            _ => None,
        }
    }

    /// Consumes the MMC5 "background CHR window changed" flag (the 8x16-sprite
    /// latch flipped). Always `false` for other mappers.
    pub(crate) fn take_chr_bg_dirty(&mut self) -> bool {
        match self {
            Self::Mmc5(mapper) => mapper.take_chr_bg_dirty(),
            _ => false,
        }
    }

    pub(crate) fn mirroring_override(&self) -> Option<NametableMirroring> {
        match self {
            Self::Axrom(mapper) => Some(mapper.mirroring()),
            Self::Mmc3(mapper) => Some(mapper.mirroring()),
            Self::Fme7(mapper) => Some(mapper.mirroring()),
            Self::Mmc2(mapper) => Some(mapper.mirroring()),
            Self::Mmc4(mapper) => Some(mapper.mirroring()),
            Self::Mmc5(mapper) => Some(mapper.mirroring()),
            _ => None,
        }
    }

    /// Whether this mapper wants per-pattern-fetch CHR notifications (the
    /// MMC2/MMC4 tile latch). Used to gate PPU CHR-fetch recording so
    /// non-latching mappers pay zero cost in the render path.
    #[must_use]
    pub(crate) fn wants_chr_fetch_notify(&self) -> bool {
        matches!(self, Self::Mmc2(_) | Self::Mmc4(_))
    }

    /// Forwards a PPU pattern-table fetch address to the mapper's CHR latch.
    /// Returns `true` when the mapper's CHR window changed and must be re-synced
    /// to the PPU. All non-latching mappers are a no-op returning `false`.
    #[must_use]
    pub(crate) fn notify_ppu_chr_fetch(&mut self, addr: u16) -> bool {
        match self {
            Self::Mmc2(mapper) => mapper.notify_ppu_chr_fetch(addr),
            Self::Mmc4(mapper) => mapper.notify_ppu_chr_fetch(addr),
            _ => false,
        }
    }

    pub(crate) fn irq_pending(&self) -> bool {
        match self {
            Self::Mmc3(mapper) => mapper.irq_pending(),
            Self::Fme7(mapper) => mapper.irq_pending(),
            Self::Mmc5(mapper) => mapper.irq_pending(),
            _ => false,
        }
    }

    pub(crate) fn on_ppu_dot(
        &mut self,
        scanline: u16,
        dot: u16,
        rendering_enabled: bool,
        ppu_ctrl: u8,
    ) {
        match self {
            Self::Mmc3(mapper) => mapper.on_ppu_dot(scanline, dot, rendering_enabled, ppu_ctrl),
            Self::Fme7(mapper) => mapper.on_ppu_dot(scanline, dot, rendering_enabled, ppu_ctrl),
            Self::Mmc5(mapper) => mapper.on_ppu_dot(scanline, dot, rendering_enabled, ppu_ctrl),
            _ => {}
        }
    }

    /// Whether this mapper exposes CPU-visible registers/RAM in the
    /// `$5000..=$5FFF` expansion window (MMC5). Gates the expansion image sync so
    /// other mappers pay no cost.
    #[must_use]
    pub(crate) fn has_expansion(&self) -> bool {
        matches!(self, Self::Mmc5(_))
    }

    /// Handles a CPU write to `$5000..=$5FFF`. Returns `true` when the mapper
    /// consumed it (so the core re-syncs PRG/CHR/mirroring and the expansion
    /// image); `false` for mappers without expansion registers.
    pub(crate) fn write_expansion(&mut self, addr: u16, value: u8) -> bool {
        match self {
            Self::Mmc5(mapper) => {
                mapper.write_expansion(addr, value);
                true
            }
            _ => false,
        }
    }

    /// Returns the byte a CPU read of `$5000..=$5FFF` observes, or `None` for
    /// write-only / unmapped addresses.
    #[must_use]
    pub(crate) fn expansion_read(&self, addr: u16) -> Option<u8> {
        match self {
            Self::Mmc5(mapper) => mapper.expansion_read(addr),
            _ => None,
        }
    }

    /// Applies the side effect of a CPU read of `$5000..=$5FFF` (MMC5 `$5204`
    /// clears the pending IRQ flag).
    pub(crate) fn on_expansion_read(&mut self, addr: u16) {
        if let Self::Mmc5(mapper) = self {
            mapper.on_expansion_read(addr);
        }
    }

    pub(crate) fn sync_chr_ram_from_ppu_window(&mut self, window: &[u8; CHR_8K_BYTES]) {
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
            Self::Mmc5(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            _ => {}
        }
    }

    #[must_use]
    pub(crate) fn chr_writable(&self) -> bool {
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
            Self::Mmc5(mapper) => mapper.chr_writable(),
            _ => false,
        }
    }

    pub(crate) fn delta_to(&self, after: &Self) -> Option<MapperDelta> {
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
            (Self::Mmc5(before), Self::Mmc5(after)) => {
                let state = after.state();
                (before.state() != state).then_some(MapperDeltaKind::Mmc5(state))
            }
            _ => Some(MapperDeltaKind::Replace(Some(after.clone()))),
        }?;

        Some(MapperDelta { kind })
    }

    #[must_use]
    pub(crate) fn snapshot_delta(&self) -> Option<MapperDelta> {
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
            Self::Mmc5(mapper) => MapperDeltaKind::Mmc5(mapper.state()),
        };

        Some(MapperDelta { kind })
    }

    pub(crate) fn apply_delta(&mut self, delta: &MapperDelta, chr_window: &[u8; CHR_8K_BYTES]) {
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
            MapperDeltaKind::Mmc5(state) => {
                let Self::Mmc5(mapper) = self else {
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
