//! Provides types and functions for determining where CPU memory accesses
//! are routed.
//!
//! The NES has a 16-bit address space (0x0000 to 0xFFFF). This module
//! defines the distinct regions of that address space and how to map
//! an address to its region.

/// Represents the distinct regions of the NES CPU memory map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusRegion {
    /// CPU RAM region (`0x0000..=0x1FFF`).
    CpuRam,
    /// PPU Registers region (`0x2000..=0x3FFF`).
    PpuRegisters,
    /// APU and I/O Registers region (`0x4000..=0x4017`).
    ApuIo,
    /// Disabled APU and I/O Registers region (`0x4018..=0x401F`).
    DisabledIo,
    /// Cartridge Expansion ROM region (`0x4020..=0x5FFF`).
    CartridgeExpansion,
    /// Cartridge PRG RAM region (`0x6000..=0x7FFF`).
    CartridgePrgRam,
    /// Cartridge PRG ROM region (`0x8000..=0xFFFF`).
    CartridgePrgRom,
}

impl BusRegion {
    /// Returns `true` if this region is considered "legal" for normal access.
    /// This currently returns `true` for all regions, but could be used
    /// in the future to identify open bus or strictly-invalid access zones.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::bus::BusRegion;
    ///
    /// assert!(BusRegion::CpuRam.is_legal());
    /// ```
    #[must_use]
    pub fn is_legal(self) -> bool {
        true
    }
}

/// Maps a 16-bit CPU address to its corresponding [`BusRegion`].
///
/// ## Panics
///
/// This function does not panic.
///
/// ## Examples
///
/// ```
/// use nes_core::bus::{map_region, BusRegion};
///
/// assert_eq!(map_region(0x0000), BusRegion::CpuRam);
/// assert_eq!(map_region(0x2000), BusRegion::PpuRegisters);
/// assert_eq!(map_region(0x8000), BusRegion::CartridgePrgRom);
/// ```
#[must_use]
pub fn map_region(addr: u16) -> BusRegion {
    match addr {
        0x0000..=0x1FFF => BusRegion::CpuRam,
        0x2000..=0x3FFF => BusRegion::PpuRegisters,
        0x4000..=0x4017 => BusRegion::ApuIo,
        0x4018..=0x401F => BusRegion::DisabledIo,
        0x4020..=0x5FFF => BusRegion::CartridgeExpansion,
        0x6000..=0x7FFF => BusRegion::CartridgePrgRam,
        _ => BusRegion::CartridgePrgRom,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_region_returns_correct_region() {
        // CpuRam: 0x0000..=0x1FFF
        assert_eq!(map_region(0x0000), BusRegion::CpuRam);
        assert_eq!(map_region(0x1FFF), BusRegion::CpuRam);

        // PpuRegisters: 0x2000..=0x3FFF
        assert_eq!(map_region(0x2000), BusRegion::PpuRegisters);
        assert_eq!(map_region(0x3FFF), BusRegion::PpuRegisters);

        // ApuIo: 0x4000..=0x4017
        assert_eq!(map_region(0x4000), BusRegion::ApuIo);
        assert_eq!(map_region(0x4017), BusRegion::ApuIo);

        // DisabledIo: 0x4018..=0x401F
        assert_eq!(map_region(0x4018), BusRegion::DisabledIo);
        assert_eq!(map_region(0x401F), BusRegion::DisabledIo);

        // CartridgeExpansion: 0x4020..=0x5FFF
        assert_eq!(map_region(0x4020), BusRegion::CartridgeExpansion);
        assert_eq!(map_region(0x5FFF), BusRegion::CartridgeExpansion);

        // CartridgePrgRam: 0x6000..=0x7FFF
        assert_eq!(map_region(0x6000), BusRegion::CartridgePrgRam);
        assert_eq!(map_region(0x7FFF), BusRegion::CartridgePrgRam);

        // CartridgePrgRom: _
        assert_eq!(map_region(0x8000), BusRegion::CartridgePrgRom);
        assert_eq!(map_region(0xFFFF), BusRegion::CartridgePrgRom);
    }
}
