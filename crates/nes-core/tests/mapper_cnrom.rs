use nes_core::mapper::Cnrom;

#[test]
fn cnrom_prg_mapping_is_fixed_across_chr_bank_switches() {
    let mut prg = vec![0_u8; 32 * 1024];
    prg[0] = 0x11;
    prg[16 * 1024] = 0x22;

    let mut chr = vec![0_u8; 16 * 1024];
    chr[..8 * 1024].fill(1);
    chr[8 * 1024..].fill(2);

    let mut mapper = Cnrom::from_prg_chr(prg, chr);
    let before = mapper.read_prg(0x8000);
    mapper.write_prg(0x8000, 1);
    assert_eq!(before, mapper.read_prg(0x8000));
}

#[test]
fn cnrom_write_selects_chr_bank_window() {
    let prg = vec![0_u8; 32 * 1024];
    let mut chr = vec![0_u8; 16 * 1024];
    chr[..8 * 1024].fill(0xAA);
    chr[8 * 1024..].fill(0xBB);

    let mut mapper = Cnrom::from_prg_chr(prg, chr);
    assert_eq!(mapper.selected_chr_bank(), 0);
    assert_eq!(mapper.chr_window()[0], 0xAA);

    mapper.write_prg(0x8000, 1);

    assert_eq!(mapper.selected_chr_bank(), 1);
    assert_eq!(mapper.chr_window()[0], 0xBB);
}
