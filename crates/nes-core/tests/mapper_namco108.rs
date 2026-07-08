use nes_core::mapper::Namco108;

const PRG_8K: usize = 8 * 1024;
const CHR_1K: usize = 1024;

fn prg_with_bank_markers(banks: usize) -> Vec<u8> {
    let mut prg = vec![0_u8; banks * PRG_8K];
    for bank in 0..banks {
        prg[bank * PRG_8K] = bank as u8;
    }
    prg
}

fn chr_with_bank_markers(banks: usize) -> Vec<u8> {
    let mut chr = vec![0_u8; banks * CHR_1K];
    for bank in 0..banks {
        chr[bank * CHR_1K] = bank as u8;
    }
    chr
}

fn select_register(m: &mut Namco108, index: u8, value: u8) {
    m.write_prg(0x8000, index); // even: bank select (bits 0-2)
    m.write_prg(0x8001, value); // odd: bank data
}

#[test]
fn namco108_prg_switches_r6_r7_with_fixed_high_windows() {
    let mut mapper = Namco108::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
    select_register(&mut mapper, 6, 3);
    select_register(&mut mapper, 7, 4);

    assert_eq!(mapper.read_prg(0x8000), 3); // R6
    assert_eq!(mapper.read_prg(0xA000), 4); // R7
    assert_eq!(mapper.read_prg(0xC000), 6); // bankcount - 2 (fixed)
    assert_eq!(mapper.read_prg(0xE000), 7); // bankcount - 1 (fixed)
}

#[test]
fn namco108_2k_chr_banks_via_r0_r1_ignore_bit0() {
    let mut mapper = Namco108::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
    select_register(&mut mapper, 0, 3); // 3 -> 2 (bit 0 ignored) => banks 2,3
    select_register(&mut mapper, 1, 5); // 5 -> 4 => banks 4,5

    let window = mapper.chr_window();
    assert_eq!(window[0x0000], 2);
    assert_eq!(window[0x0400], 3);
    assert_eq!(window[0x0800], 4);
    assert_eq!(window[0x0C00], 5);
}

#[test]
fn namco108_1k_chr_banks_via_r2_r5() {
    let mut mapper = Namco108::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
    select_register(&mut mapper, 2, 7);
    select_register(&mut mapper, 3, 6);
    select_register(&mut mapper, 4, 5);
    select_register(&mut mapper, 5, 4);

    let window = mapper.chr_window();
    assert_eq!(window[0x1000], 7);
    assert_eq!(window[0x1400], 6);
    assert_eq!(window[0x1800], 5);
    assert_eq!(window[0x1C00], 4);
}

#[test]
fn namco108_bank_select_uses_only_index_bits() {
    let mut mapper = Namco108::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(8));
    // Upper bits of the select value are ignored: 0xFE & 0x07 = 6.
    mapper.write_prg(0x8000, 0xFE);
    mapper.write_prg(0x8001, 2);
    assert_eq!(mapper.read_prg(0x8000), 2); // wrote R6
}

#[test]
fn namco108_register_values_masked_to_six_bits() {
    let mut mapper = Namco108::from_prg_chr(prg_with_bank_markers(64), chr_with_bank_markers(64));
    select_register(&mut mapper, 6, 0xC5); // 0xC5 & 0x3F = 5
    assert_eq!(mapper.read_prg(0x8000), 5);
}
