use crate::rom::NametableMirroring;
use serde::{Deserialize, Serialize};

use super::{
    Axrom, AxromState, Cnrom, CnromState, Gxrom, GxromState, Mmc1, Mmc1State, Mmc3, Mmc3State,
    Nrom, Uxrom, UxromState,
};

// Locally defined constant instead of importing from api.rs to avoid circularity/refactor issues
const CHR_8K_BYTES: usize = 8 * 1024;

/// Opaque mapper-runtime delta between two `CoreSnapshot`s.
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
        }
    }

    pub(crate) fn chr_window(&self) -> Option<([u8; CHR_8K_BYTES], bool)> {
        match self {
            Self::Mmc3(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            Self::Cnrom(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            Self::Gxrom(mapper) => Some((mapper.chr_window(), mapper.chr_writable())),
            _ => None,
        }
    }

    pub(crate) fn mirroring_override(&self) -> Option<NametableMirroring> {
        match self {
            Self::Axrom(mapper) => Some(mapper.mirroring()),
            Self::Mmc3(mapper) => Some(mapper.mirroring()),
            _ => None,
        }
    }

    pub(crate) fn irq_pending(&self) -> bool {
        match self {
            Self::Mmc3(mapper) => mapper.irq_pending(),
            _ => false,
        }
    }

    pub(crate) fn on_ppu_dot(&mut self, scanline: u16, dot: u16, rendering_enabled: bool, ppu_ctrl: u8) {
        if let Self::Mmc3(mapper) = self {
            mapper.on_ppu_dot(scanline, dot, rendering_enabled, ppu_ctrl);
        }
    }

    pub(crate) fn sync_chr_ram_from_ppu_window(&mut self, window: &[u8; CHR_8K_BYTES]) {
        match self {
            Self::Cnrom(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            Self::Gxrom(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            Self::Mmc3(mapper) => mapper.sync_chr_ram_from_ppu_window(window),
            _ => {}
        }
    }

    #[must_use]
    pub(crate) fn chr_writable(&self) -> bool {
        match self {
            Self::Cnrom(mapper) => mapper.chr_writable(),
            Self::Gxrom(mapper) => mapper.chr_writable(),
            Self::Mmc3(mapper) => mapper.chr_writable(),
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
                mapper.restore_state(*state);
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
