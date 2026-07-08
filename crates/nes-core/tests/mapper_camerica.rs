use nes_core::mapper::Camerica;

const PRG_16K: usize = 16 * 1024;
const CHR_8K: usize = 8 * 1024;

fn prg_with_bank_markers(banks: usize) -> Vec<u8> {
    let mut prg = vec![0_u8; banks * PRG_16K];
    for bank in 0..banks {
        prg[bank * PRG_16K] = 0x10 + bank as u8;
    }
    prg
}

#[test]
fn camerica_switches_16k_bank_at_8000() {
    let mut mapper = Camerica::from_prg_rom(prg_with_bank_markers(4));
    assert_eq!(mapper.read_prg(0x8000), 0x10);

    mapper.write_prg(0xC000, 0x02); // bank register lives in $C000-$FFFF
    assert_eq!(mapper.read_prg(0x8000), 0x12);
}

#[test]
fn camerica_high_window_is_fixed_to_last_bank() {
    let mut mapper = Camerica::from_prg_rom(prg_with_bank_markers(4));
    assert_eq!(mapper.read_prg(0xC000), 0x13); // last bank (index 3)

    mapper.write_prg(0xE000, 0x01); // switch low window only
    assert_eq!(mapper.read_prg(0x8000), 0x11);
    assert_eq!(mapper.read_prg(0xC000), 0x13); // high window unchanged
}

#[test]
fn camerica_masks_bank_select_to_low_nibble() {
    let mut mapper = Camerica::from_prg_rom(prg_with_bank_markers(4));
    mapper.write_prg(0xD000, 0xF2); // 0xF2 & 0x0F = 2
    assert_eq!(mapper.read_prg(0x8000), 0x12);
}

#[test]
fn camerica_ignores_writes_to_8000_bfff() {
    let mut mapper = Camerica::from_prg_rom(prg_with_bank_markers(4));
    mapper.write_prg(0xC000, 0x02);
    mapper.write_prg(0x8000, 0x03); // ignored
    mapper.write_prg(0xBFFF, 0x01); // ignored
    assert_eq!(mapper.read_prg(0x8000), 0x12);
}

#[test]
fn camerica_exposes_writable_chr_ram() {
    let mut mapper = Camerica::from_prg_rom(prg_with_bank_markers(2));
    assert!(mapper.chr_writable());

    let mut window = [0_u8; CHR_8K];
    window[7] = 0x99;
    mapper.sync_chr_ram_from_ppu_window(&window);
    assert_eq!(mapper.chr_window()[7], 0x99);
}
