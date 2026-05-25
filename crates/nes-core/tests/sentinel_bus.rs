use nes_core::bus::{BusRegion, map_region};

#[test]
fn test_bus_region_default_mutant() {
    // Tests map_region inside regions to ensure Default::default() (CpuRam) mutant fails
    assert_eq!(map_region(0x1000), BusRegion::CpuRam);
    assert_eq!(map_region(0x3000), BusRegion::PpuRegisters);
    assert_eq!(map_region(0x4008), BusRegion::ApuIo);
    assert_eq!(map_region(0x401A), BusRegion::DisabledIo);
    assert_eq!(map_region(0x5000), BusRegion::CartridgeExpansion);
    assert_eq!(map_region(0x7000), BusRegion::CartridgePrgRam);
    assert_eq!(map_region(0xC000), BusRegion::CartridgePrgRom);
}
