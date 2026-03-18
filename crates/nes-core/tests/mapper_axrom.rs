use nes_core::mapper::Axrom;

#[test]
fn axrom_write_selects_bank_and_one_screen_mirroring() {
    let mut prg = vec![0_u8; 64 * 1024];
    prg[..32 * 1024].fill(0x11);
    prg[32 * 1024..].fill(0x22);

    let mut mapper = Axrom::from_prg_rom(prg);
    assert_eq!(mapper.selected_bank(), 0);
    assert_eq!(mapper.selected_nametable_bank(), 0);
    assert_eq!(mapper.read_prg(0x8000), 0x11);

    mapper.write_prg(0x8000, 0x11);

    assert_eq!(mapper.selected_bank(), 1);
    assert_eq!(mapper.selected_nametable_bank(), 1);
    assert_eq!(mapper.read_prg(0x8000), 0x22);
}

#[test]
fn axrom_selected_bank_maps_entire_32k_prg_window() {
    let mut prg = vec![0_u8; 64 * 1024];
    prg[0x0001] = 0x10;
    prg[0x4001] = 0x11;
    prg[32 * 1024 + 0x0001] = 0x20;
    prg[32 * 1024 + 0x4001] = 0x21;

    let mut mapper = Axrom::from_prg_rom(prg);
    assert_eq!(mapper.read_prg(0x8001), 0x10);
    assert_eq!(mapper.read_prg(0xC001), 0x11);

    mapper.write_prg(0x8000, 0x01);

    assert_eq!(mapper.read_prg(0x8001), 0x20);
    assert_eq!(mapper.read_prg(0xC001), 0x21);
}

#[test]
fn axrom_short_prg_is_zero_padded_to_full_32k_window() {
    let mapper = Axrom::from_prg_rom(vec![0xAB, 0xCD]);

    assert_eq!(mapper.read_prg(0x8000), 0xAB);
    assert_eq!(mapper.read_prg(0x8001), 0xCD);
    assert_eq!(mapper.read_prg(0x8002), 0x00);
    assert_eq!(mapper.read_prg(0xFFFF), 0x00);
}

#[test]
fn axrom_read_prg_masks_address_into_32k_window() {
    let mut prg = vec![0_u8; 32 * 1024];
    prg[0x1234] = 0x5A;
    let mapper = Axrom::from_prg_rom(prg);

    assert_eq!(mapper.read_prg(0x9234), 0x5A);
    assert_eq!(mapper.read_prg(0x1234), 0x5A);
}

#[test]
fn axrom_state_can_be_saved_and_restored() {
    let mut mapper = nes_core::mapper::Axrom::from_prg_rom(vec![0; 4 * 32 * 1024]);
    mapper.write_prg(0x8000, 0x12); // PRG bank 2, NT bank 1

    let state = mapper.state();
    assert_eq!(mapper.selected_bank(), 2);
    assert_eq!(mapper.selected_nametable_bank(), 1);

    mapper.write_prg(0x8000, 0x03); // PRG bank 3, NT bank 0
    assert_eq!(mapper.selected_bank(), 3);
    assert_eq!(mapper.selected_nametable_bank(), 0);

    mapper.restore_state(state);
    assert_eq!(mapper.selected_bank(), 2);
    assert_eq!(mapper.selected_nametable_bank(), 1);
}

#[test]
fn axrom_read_prg_through_trait() {
    use nes_core::mapper::Mapper;
    let mut prg = vec![0_u8; 32 * 1024];
    prg[0x0000] = 0xAA;
    let mapper = nes_core::mapper::Axrom::from_prg_rom(prg);

    // Explicitly test <Self as Mapper>::read_prg
    assert_eq!(
        <nes_core::mapper::Axrom as Mapper>::read_prg(&mapper, 0x8000),
        mapper.read_prg(0x8000)
    );
}
