use nes_core::bus::{map_region, BusRegion};

#[test]
fn bus_region_is_legal_strict() {
    assert_eq!(map_region(0x0000), BusRegion::CpuRam);
    assert_eq!(map_region(0x0FFF), BusRegion::CpuRam);
    assert_eq!(map_region(0x1FFF), BusRegion::CpuRam);

    assert_eq!(map_region(0x2000), BusRegion::PpuRegisters);
    assert_eq!(map_region(0x3000), BusRegion::PpuRegisters);
    assert_eq!(map_region(0x3FFF), BusRegion::PpuRegisters);

    assert_eq!(map_region(0x4000), BusRegion::ApuIo);
    assert_eq!(map_region(0x400F), BusRegion::ApuIo);
    assert_eq!(map_region(0x4017), BusRegion::ApuIo);

    assert_eq!(map_region(0x4018), BusRegion::DisabledIo);
    assert_eq!(map_region(0x401A), BusRegion::DisabledIo);
    assert_eq!(map_region(0x401F), BusRegion::DisabledIo);

    assert_eq!(map_region(0x4020), BusRegion::CartridgeExpansion);
    assert_eq!(map_region(0x5000), BusRegion::CartridgeExpansion);
    assert_eq!(map_region(0x5FFF), BusRegion::CartridgeExpansion);

    assert_eq!(map_region(0x6000), BusRegion::CartridgePrgRam);
    assert_eq!(map_region(0x7000), BusRegion::CartridgePrgRam);
    assert_eq!(map_region(0x7FFF), BusRegion::CartridgePrgRam);

    assert_eq!(map_region(0x8000), BusRegion::CartridgePrgRom);
    assert_eq!(map_region(0xC000), BusRegion::CartridgePrgRom);
    assert_eq!(map_region(0xFFFF), BusRegion::CartridgePrgRom);

    assert!(BusRegion::CpuRam.is_legal());
    assert!(BusRegion::PpuRegisters.is_legal());
    assert!(BusRegion::ApuIo.is_legal());
    assert!(BusRegion::DisabledIo.is_legal());
    assert!(BusRegion::CartridgeExpansion.is_legal());
    assert!(BusRegion::CartridgePrgRam.is_legal());
    assert!(BusRegion::CartridgePrgRom.is_legal());
}
