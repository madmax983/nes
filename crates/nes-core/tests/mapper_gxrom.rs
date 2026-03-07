use nes_core::mapper::Gxrom;

#[test]
fn gxrom_write_selects_prg_and_chr_banks() {
    let mut prg = vec![0_u8; 4 * 32 * 1024];
    prg[0x0000] = 0x10;
    prg[32 * 1024] = 0x20;
    prg[2 * 32 * 1024] = 0x30;
    prg[3 * 32 * 1024] = 0x40;

    let mut chr = vec![0_u8; 4 * 8 * 1024];
    chr[0x0000] = 0xA0;
    chr[8 * 1024] = 0xB0;
    chr[2 * 8 * 1024] = 0xC0;
    chr[3 * 8 * 1024] = 0xD0;

    let mut mapper = Gxrom::from_prg_chr(prg, chr);
    assert_eq!(mapper.selected_prg_bank(), 0);
    assert_eq!(mapper.selected_chr_bank(), 0);
    assert_eq!(mapper.read_prg(0x8000), 0x10);
    assert_eq!(mapper.chr_window()[0], 0xA0);

    mapper.write_prg(0x8000, 0x21);

    assert_eq!(mapper.selected_prg_bank(), 2);
    assert_eq!(mapper.selected_chr_bank(), 1);
    assert_eq!(mapper.read_prg(0x8000), 0x30);
    assert_eq!(mapper.chr_window()[0], 0xB0);
}

#[test]
fn gxrom_selected_prg_bank_maps_entire_32k_window() {
    let mut prg = vec![0_u8; 2 * 32 * 1024];
    prg[0x0001] = 0x11;
    prg[0x4001] = 0x12;
    prg[32 * 1024 + 0x0001] = 0x21;
    prg[32 * 1024 + 0x4001] = 0x22;

    let chr = vec![0_u8; 8 * 1024];
    let mut mapper = Gxrom::from_prg_chr(prg, chr);

    assert_eq!(mapper.read_prg(0x8001), 0x11);
    assert_eq!(mapper.read_prg(0xC001), 0x12);

    mapper.write_prg(0x8000, 0x10);

    assert_eq!(mapper.read_prg(0x8001), 0x21);
    assert_eq!(mapper.read_prg(0xC001), 0x22);
}

#[test]
fn gxrom_short_inputs_are_zero_padded() {
    let prg = vec![0xAB, 0xCD];
    let chr = vec![0x5A_u8; (8 * 1024) - 2];
    let mapper = Gxrom::from_prg_chr(prg, chr);

    assert_eq!(mapper.read_prg(0x8000), 0xAB);
    assert_eq!(mapper.read_prg(0x8001), 0xCD);
    assert_eq!(mapper.read_prg(0x8002), 0x00);
    assert_eq!(mapper.read_prg(0xFFFF), 0x00);

    let window = mapper.chr_window();
    assert_eq!(window[(8 * 1024) - 3], 0x5A);
    assert_eq!(window[(8 * 1024) - 2], 0x00);
    assert_eq!(window[(8 * 1024) - 1], 0x00);
}

#[test]
fn gxrom_read_prg_masks_address_into_32k_window() {
    let mut prg = vec![0_u8; 32 * 1024];
    prg[0x1234] = 0x5A;
    let mapper = Gxrom::from_prg_chr(prg, vec![0_u8; 8 * 1024]);

    assert_eq!(mapper.read_prg(0x9234), 0x5A);
    assert_eq!(mapper.read_prg(0x1234), 0x5A);
}

#[test]
fn gxrom_bank_counts_do_not_wrap_at_256_banks() {
    let mut prg = vec![0_u8; 256 * 32 * 1024];
    prg[0x0000] = 0xA0;
    prg[32 * 1024] = 0xB0;

    let mut chr = vec![0_u8; 256 * 8 * 1024];
    chr[0x0000] = 0xC0;
    chr[8 * 1024] = 0xD0;

    let mut mapper = Gxrom::from_prg_chr(prg, chr);
    assert_eq!(mapper.read_prg(0x8000), 0xA0);
    assert_eq!(mapper.chr_window()[0], 0xC0);

    mapper.write_prg(0x8000, 0x11);

    assert_eq!(mapper.selected_prg_bank(), 1);
    assert_eq!(mapper.selected_chr_bank(), 1);
    assert_eq!(mapper.read_prg(0x8000), 0xB0);
    assert_eq!(mapper.chr_window()[0], 0xD0);
}
