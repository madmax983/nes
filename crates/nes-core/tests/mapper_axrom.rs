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
