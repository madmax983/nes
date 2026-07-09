use nes_core::mapper::Mmc4;

const PRG_16K: usize = 16 * 1024;
const CHR_4K: usize = 4 * 1024;

fn prg_with_bank_markers(banks_16k: usize) -> Vec<u8> {
    let mut prg = vec![0_u8; banks_16k * PRG_16K];
    for bank in 0..banks_16k {
        prg[bank * PRG_16K] = bank as u8;
    }
    prg
}

fn chr_with_bank_markers(banks_4k: usize) -> Vec<u8> {
    let mut chr = vec![0_u8; banks_4k * CHR_4K];
    for bank in 0..banks_4k {
        chr[bank * CHR_4K] = bank as u8;
    }
    chr
}

#[test]
fn mmc4_switchable_16k_bank_and_fixed_last() {
    let mut m = Mmc4::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(4));
    // Power-on: the switchable 16KB window ($8000-$BFFF) is bank 0; the fixed
    // window ($C000-$FFFF) is the last bank (3). Each bank marker sits at the
    // start of its 16KB bank.
    assert_eq!(m.read_prg(0x8000), 0);
    assert_eq!(m.read_prg(0xC000), 3);

    m.write_prg(0xA000, 2); // switchable window -> bank 2
    assert_eq!(m.read_prg(0x8000), 2);
    assert_eq!(m.read_prg(0xC000), 3); // fixed last unchanged
}

#[test]
fn mmc4_prg_bank_select_masks_to_four_bits() {
    let mut m = Mmc4::from_prg_chr(prg_with_bank_markers(16), chr_with_bank_markers(4));
    m.write_prg(0xA000, 0xF5); // 0xF5 & 0x0F = 5
    assert_eq!(m.read_prg(0x8000), 5);
}

#[test]
fn mmc4_prg_ram_round_trip() {
    let mut m = Mmc4::from_prg_chr(prg_with_bank_markers(2), chr_with_bank_markers(4));
    // $6000-$7FFF PRG-RAM is always enabled on MMC4.
    m.write_prg(0x6000, 0xAB);
    m.write_prg(0x7FFF, 0xCD);
    assert_eq!(m.read_prg(0x6000), 0xAB);
    assert_eq!(m.read_prg(0x7FFF), 0xCD);

    // The CPU-bus PRG-RAM route sees the same storage.
    assert_eq!(m.read_prg_ram(0x6000), Some(0xAB));
    assert_eq!(m.read_prg_ram(0x8000), None); // outside the window
    m.write_prg_ram(0x6001, 0x42);
    assert_eq!(m.read_prg(0x6001), 0x42);
}

#[test]
fn mmc4_chr_latch_matches_mmc2_low_and_high_halves() {
    let mut m = Mmc4::from_prg_chr(prg_with_bank_markers(2), chr_with_bank_markers(8));
    m.write_prg(0xB000, 1); // FD low
    m.write_prg(0xC000, 4); // FE low
    m.write_prg(0xD000, 2); // FD high
    m.write_prg(0xE000, 7); // FE high

    // Default latches are FE.
    assert_eq!(m.chr_window()[0x0000], 4);
    assert_eq!(m.chr_window()[0x1000], 7);

    // Low half: non-trigger no-op, then FD/FE flips.
    assert!(!m.notify_ppu_chr_fetch(0x0100));
    assert!(m.notify_ppu_chr_fetch(0x0FD8));
    assert_eq!(m.chr_window()[0x0000], 1);
    assert!(m.notify_ppu_chr_fetch(0x0FE8));
    assert_eq!(m.chr_window()[0x0000], 4);

    // High half is independent.
    assert!(m.notify_ppu_chr_fetch(0x1FD8));
    assert_eq!(m.chr_window()[0x1000], 2);
    assert!(m.notify_ppu_chr_fetch(0x1FE8));
    assert_eq!(m.chr_window()[0x1000], 7);
    assert!(!m.notify_ppu_chr_fetch(0x1FE8)); // idempotent
}

#[test]
fn mmc4_mirroring_register_yields_two_distinct_modes() {
    let mut m = Mmc4::from_prg_chr(prg_with_bank_markers(2), chr_with_bank_markers(4));
    m.write_prg(0xF000, 0x00);
    let vertical = m.mirroring();
    m.write_prg(0xF000, 0x01);
    let horizontal = m.mirroring();
    assert_ne!(vertical, horizontal);
    m.write_prg(0xF000, 0x00);
    assert_eq!(m.mirroring(), vertical);
}

#[test]
fn mmc4_chr_rom_is_not_writable() {
    let m = Mmc4::from_prg_chr(prg_with_bank_markers(2), chr_with_bank_markers(8));
    assert!(!m.chr_writable());
}

#[test]
fn mmc4_short_inputs_do_not_panic() {
    let mut m = Mmc4::from_prg_chr(vec![0x11; PRG_16K + 9], vec![0x22; CHR_4K + 5]);
    let _ = m.read_prg(0x8000);
    let _ = m.read_prg(0xC000);
    let _ = m.read_prg(0x6000);
    let _ = m.notify_ppu_chr_fetch(0x1FD8);
    assert_eq!(m.chr_window().len(), 8 * 1024);
}

#[test]
fn mmc4_chr_ram_when_chr_absent_syncs_both_halves() {
    // Empty CHR -> writable 8KB CHR-RAM; undersized PRG is padded to whole 16KB
    // banks. The CHR-RAM write-back copies each latched 4KB half into its bank.
    let mut m = Mmc4::from_prg_chr(vec![0x11; PRG_16K + 7], vec![]);
    assert!(m.chr_writable());

    // Map the high half to a distinct 4KB bank so the two write-backs don't alias.
    m.write_prg(0xE000, 1); // FE high half -> 4KB bank 1 (low half stays bank 0)

    let mut window = [0_u8; 8 * 1024];
    window[0x0000] = 0xAA; // low 4KB half (latch0 == FE -> chr_bank_fe0)
    window[0x1000] = 0xBB; // high 4KB half (latch1 == FE -> chr_bank_fe1)
    m.sync_chr_ram_from_ppu_window(&window);

    let mapped = m.chr_window();
    assert_eq!(mapped[0x0000], 0xAA);
    assert_eq!(mapped[0x1000], 0xBB);
}

#[test]
fn mmc4_reads_and_writes_below_8000_and_unmapped_region_are_guarded() {
    let mut m = Mmc4::from_prg_chr(prg_with_bank_markers(2), chr_with_bank_markers(4));
    // $6000-$7FFF is PRG-RAM; anything below that reads open bus and ignores writes.
    assert_eq!(m.read_prg(0x0000), 0xFF);
    m.write_prg(0x4000, 0x55); // below PRG-RAM/PRG-ROM: ignored
    m.write_prg(0x8000, 0x55); // $8000-$9FFF unmapped on MMC4: ignored
    assert_eq!(m.read_prg(0x0000), 0xFF);
    assert_eq!(m.read_prg_ram(0x0000), None); // outside the $6000-$7FFF window
}

#[test]
fn mmc4_non_multiple_chr_rom_is_padded() {
    // Non-empty CHR that is not a 4KB multiple is rounded up and stays CHR-ROM.
    let m = Mmc4::from_prg_chr(prg_with_bank_markers(2), vec![0x22; 2 * CHR_4K + 5]);
    assert!(!m.chr_writable());
    assert_eq!(m.chr_window().len(), 8 * 1024);
}
