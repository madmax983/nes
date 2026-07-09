use nes_core::bus::{BusRegion, map_region};

#[test]
fn address_regions_are_unambiguous() {
    assert_eq!(map_region(0x0000), BusRegion::CpuRam);
    assert_eq!(map_region(0x1FFF), BusRegion::CpuRam);
    assert_eq!(map_region(0x2000), BusRegion::PpuRegisters);
    assert_eq!(map_region(0x3FFF), BusRegion::PpuRegisters);
    assert_eq!(map_region(0x4000), BusRegion::ApuIo);
    assert_eq!(map_region(0x4017), BusRegion::ApuIo);
    assert_eq!(map_region(0x4018), BusRegion::DisabledIo);
    assert_eq!(map_region(0x401F), BusRegion::DisabledIo);
    assert_eq!(map_region(0x4020), BusRegion::CartridgeExpansion);
    assert_eq!(map_region(0x5FFF), BusRegion::CartridgeExpansion);
    assert_eq!(map_region(0x6000), BusRegion::CartridgePrgRam);
    assert_eq!(map_region(0x7FFF), BusRegion::CartridgePrgRam);
    assert_eq!(map_region(0x8000), BusRegion::CartridgePrgRom);
    assert_eq!(map_region(0xFFFF), BusRegion::CartridgePrgRom);
}
