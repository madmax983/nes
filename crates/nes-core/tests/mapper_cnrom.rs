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

#[test]
fn cnrom_empty_prg_defaults_to_zero_byte() {
    let mapper = Cnrom::from_prg_chr(vec![], vec![0_u8; 8 * 1024]);
    assert_eq!(mapper.read_prg(0x8000), 0);
    assert_eq!(mapper.read_prg(0xFFFF), 0);
}

#[test]
fn cnrom_short_chr_input_is_zero_padded() {
    let prg = vec![0_u8; 16 * 1024];
    let chr = vec![0x5A_u8; (8 * 1024) - 3];
    let mapper = Cnrom::from_prg_chr(prg, chr);
    let window = mapper.chr_window();
    assert_eq!(window[(8 * 1024) - 4], 0x5A);
    assert_eq!(window[(8 * 1024) - 3], 0x00);
    assert_eq!(window[(8 * 1024) - 1], 0x00);
}

#[test]
fn cnrom_chr_bank_count_does_not_wrap_at_256_banks() {
    let prg = vec![0_u8; 32 * 1024];
    let mut chr = vec![0_u8; 256 * 8 * 1024];
    chr[0] = 0xA0;
    chr[8 * 1024] = 0xB0;

    let mut mapper = Cnrom::from_prg_chr(prg, chr);
    assert_eq!(mapper.chr_window()[0], 0xA0);

    mapper.write_prg(0x8000, 1);

    assert_eq!(mapper.selected_chr_bank(), 1);
    assert_eq!(mapper.chr_window()[0], 0xB0);
}
