use nes_core::mapper::ColorDreams;

const PRG_32K: usize = 32 * 1024;
const CHR_8K: usize = 8 * 1024;

fn prg_with_bank_markers(banks: usize) -> Vec<u8> {
    let mut prg = vec![0_u8; banks * PRG_32K];
    for bank in 0..banks {
        prg[bank * PRG_32K] = 0x10 + bank as u8;
    }
    prg
}

fn chr_with_bank_markers(banks: usize) -> Vec<u8> {
    let mut chr = vec![0_u8; banks * CHR_8K];
    for bank in 0..banks {
        chr[bank * CHR_8K] = 0xA0 + bank as u8;
    }
    chr
}

#[test]
fn color_dreams_switches_32k_prg_banks() {
    let mut mapper = ColorDreams::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(1));
    assert_eq!(mapper.read_prg(0x8000), 0x10);

    // Register is writable anywhere in $8000-$FFFF; bits 0-1 pick the PRG bank.
    // The bank marker lives at the start of each 32KB bank ($8000).
    mapper.write_prg(0xFFFF, 0x02);
    assert_eq!(mapper.read_prg(0x8000), 0x12);
}

#[test]
fn color_dreams_switches_8k_chr_banks() {
    let mut mapper = ColorDreams::from_prg_chr(prg_with_bank_markers(1), chr_with_bank_markers(4));
    assert_eq!(mapper.chr_window()[0], 0xA0);

    mapper.write_prg(0x8000, 0x20); // bits 4-7 = CHR bank 2
    assert_eq!(mapper.chr_window()[0], 0xA2);
}

#[test]
fn color_dreams_masks_prg_and_chr_select_bits() {
    let mut mapper = ColorDreams::from_prg_chr(prg_with_bank_markers(2), chr_with_bank_markers(2));

    // 0x1D = 0b0001_1101: PRG bits 0-1 = 01 (bank 1), CHR bits 4-7 = 0001 (bank 1).
    mapper.write_prg(0x8000, 0x1D);
    assert_eq!(mapper.read_prg(0x8000), 0x11);
    assert_eq!(mapper.chr_window()[0], 0xA1);
}

#[test]
fn color_dreams_uses_chr_ram_when_chr_absent() {
    let mut mapper = ColorDreams::from_prg_chr(prg_with_bank_markers(1), vec![]);
    assert!(mapper.chr_writable());

    let mut window = [0_u8; CHR_8K];
    window[0] = 0x42;
    mapper.sync_chr_ram_from_ppu_window(&window);
    assert_eq!(mapper.chr_window()[0], 0x42);
}

#[test]
fn color_dreams_wraps_bank_selects_modulo_available_banks() {
    // 2 PRG banks, 2 CHR banks: selecting bank 3 wraps to bank 1.
    let mut mapper = ColorDreams::from_prg_chr(prg_with_bank_markers(2), chr_with_bank_markers(2));
    mapper.write_prg(0x8000, 0x33); // PRG select 3 -> 1, CHR select 3 -> 1
    assert_eq!(mapper.read_prg(0x8000), 0x11);
    assert_eq!(mapper.chr_window()[0], 0xA1);
}
