#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusRegion {
    CpuRam,
    PpuRegisters,
    ApuIo,
    DisabledIo,
    CartridgeExpansion,
    CartridgePrgRam,
    CartridgePrgRom,
}

impl BusRegion {
    #[must_use]
    pub fn is_legal(self) -> bool {
        true
    }
}

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
