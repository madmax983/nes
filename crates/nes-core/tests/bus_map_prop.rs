use nes_core::bus::{BusRegion, map_region};

#[test]
fn every_address_maps_to_legal_region() {
    for addr in u16::MIN..=u16::MAX {
        let region = map_region(addr);
        assert!(region.is_legal(), "address 0x{addr:04X} mapped illegally");

        match addr {
            0x0000..=0x1FFF => {
                assert_eq!(region, BusRegion::CpuRam);
            }
            0x2000..=0x3FFF => {
                assert_eq!(region, BusRegion::PpuRegisters);
            }
            0x4000..=0x4017 => {
                assert_eq!(region, BusRegion::ApuIo);
            }
            0x4018..=0x401F => {
                assert_eq!(region, BusRegion::DisabledIo);
            }
            0x4020..=0x5FFF => {
                assert_eq!(region, BusRegion::CartridgeExpansion);
            }
            0x6000..=0x7FFF => {
                assert_eq!(region, BusRegion::CartridgePrgRam);
            }
            _ => {
                assert_eq!(region, BusRegion::CartridgePrgRom);
            }
        }
    }
}
