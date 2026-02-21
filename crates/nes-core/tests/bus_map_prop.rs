use nes_core::bus::map_region;

#[test]
fn every_address_maps_to_legal_region() {
    for addr in u16::MIN..=u16::MAX {
        let region = map_region(addr);
        assert!(region.is_legal(), "address 0x{addr:04X} mapped illegally");
    }
}
