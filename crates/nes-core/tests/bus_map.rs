use nes_core::bus::{BusRegion, map_region};

#[test]
fn address_regions_are_unambiguous() {
    let region = map_region(0x8000);
    assert_eq!(region, BusRegion::CartridgePrgRom);
}
