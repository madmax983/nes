use nes_core::mapper::Mmc2;
use nes_core::{Command, NesCore};

const PRG_8K: usize = 8 * 1024;
const CHR_4K: usize = 4 * 1024;

fn prg_with_bank_markers(banks_8k: usize) -> Vec<u8> {
    let mut prg = vec![0_u8; banks_8k * PRG_8K];
    for bank in 0..banks_8k {
        prg[bank * PRG_8K] = bank as u8;
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
fn mmc2_switchable_low_bank_and_fixed_last_three() {
    let mut m = Mmc2::from_prg_chr(prg_with_bank_markers(8), chr_with_bank_markers(4));
    // Power-on: switchable $8000 bank is 0; the top three slots are the last
    // three banks (5, 6, 7 for an 8-bank ROM).
    assert_eq!(m.read_prg(0x8000), 0);
    assert_eq!(m.read_prg(0xA000), 5);
    assert_eq!(m.read_prg(0xC000), 6);
    assert_eq!(m.read_prg(0xE000), 7);

    // $A000 reg selects the $8000-$9FFF bank; the fixed banks never move.
    m.write_prg(0xA000, 4);
    assert_eq!(m.read_prg(0x8000), 4);
    assert_eq!(m.read_prg(0xA000), 5);
    assert_eq!(m.read_prg(0xC000), 6);
    assert_eq!(m.read_prg(0xE000), 7);
}

#[test]
fn mmc2_prg_bank_select_masks_to_four_bits() {
    let mut m = Mmc2::from_prg_chr(prg_with_bank_markers(16), chr_with_bank_markers(4));
    m.write_prg(0xA000, 0xF9); // 0xF9 & 0x0F = 9
    assert_eq!(m.read_prg(0x8000), 9);
}

#[test]
fn mmc2_low_half_chr_latch_switches_on_fd_fe_fetches() {
    let mut m = Mmc2::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
    m.write_prg(0xB000, 3); // low-half bank used when latch0 == FD
    m.write_prg(0xC000, 6); // low-half bank used when latch0 == FE

    // Latches power up at FE, so the low half shows the $C000 bank (6). A
    // non-trigger low-table fetch does not change the latch.
    assert!(!m.notify_ppu_chr_fetch(0x0000));
    assert_eq!(m.chr_window()[0x0000], 6);

    // Fetching tile $FD ($0FD8) flips latch0 -> FD; the change is reported.
    assert!(m.notify_ppu_chr_fetch(0x0FD8));
    assert_eq!(m.chr_window()[0x0000], 3);
    // Re-triggering the same latch value reports no change.
    assert!(!m.notify_ppu_chr_fetch(0x0FD8));

    // Fetching tile $FE ($0FE8) flips latch0 back -> FE.
    assert!(m.notify_ppu_chr_fetch(0x0FE8));
    assert_eq!(m.chr_window()[0x0000], 6);

    // Any fine-Y offset within the trigger tile still latches (matched on & 0x1FF8).
    assert!(m.notify_ppu_chr_fetch(0x0FDF));
    assert_eq!(m.chr_window()[0x0000], 3);
}

#[test]
fn mmc2_high_half_chr_latch_is_independent() {
    let mut m = Mmc2::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
    m.write_prg(0xD000, 2); // high-half bank when latch1 == FD
    m.write_prg(0xE000, 7); // high-half bank when latch1 == FE

    // Default latch1 == FE -> high half is the $E000 bank (7).
    assert_eq!(m.chr_window()[0x1000], 7);

    // Low-table triggers must not disturb the high-half latch.
    assert!(m.notify_ppu_chr_fetch(0x0FD8));
    assert_eq!(m.chr_window()[0x1000], 7);

    // High-table triggers flip latch1 only.
    assert!(m.notify_ppu_chr_fetch(0x1FD8));
    assert_eq!(m.chr_window()[0x1000], 2);
    assert!(m.notify_ppu_chr_fetch(0x1FE8));
    assert_eq!(m.chr_window()[0x1000], 7);
}

#[test]
fn mmc2_mirroring_register_yields_two_distinct_modes() {
    // rom::NametableMirroring is not publicly re-exported, so assert the two
    // $F000 values map to two distinct modes without naming the enum.
    let mut m = Mmc2::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
    m.write_prg(0xF000, 0x00);
    let vertical = m.mirroring();
    m.write_prg(0xF000, 0x01);
    let horizontal = m.mirroring();
    assert_ne!(vertical, horizontal);
    m.write_prg(0xF000, 0x00);
    assert_eq!(m.mirroring(), vertical);
}

#[test]
fn mmc2_chr_rom_is_not_writable() {
    let m = Mmc2::from_prg_chr(prg_with_bank_markers(4), chr_with_bank_markers(8));
    assert!(!m.chr_writable());
}

#[test]
fn mmc2_short_inputs_do_not_panic() {
    // Undersized PRG/CHR are padded up; reads and window assembly never panic.
    let mut m = Mmc2::from_prg_chr(vec![0x11; PRG_8K + 7], vec![0x22; CHR_4K + 3]);
    let _ = m.read_prg(0x8000);
    let _ = m.read_prg(0xE000);
    let _ = m.notify_ppu_chr_fetch(0x0FD8);
    assert_eq!(m.chr_window().len(), 8 * 1024);
}

// --- NesCore-level end-to-end CHR-latch integration -------------------------
//
// These tests drive a synthetic MMC2 ROM through the full PPU render path and
// verify that the CHR tile latch actually re-banks the background mid-frame.
// The tile-latch trigger only reaches the mapper through the PPU CHR-fetch hook
// wired into `NesCore`, so a passing result exercises the integration end to
// end (not just the mapper's `notify_ppu_chr_fetch` in isolation).

/// Builds a 32KB-PRG / 8KB-CHR MMC2 iNES image.
///
/// CHR is two 4KB banks. In both banks tile `$01`'s pattern is a solid color
/// index, but a *different* index per bank, so the palette makes the two banks
/// visually distinguishable:
/// * 4KB bank 0 -> tile `$01` = color index 1
/// * 4KB bank 1 -> tile `$01` = color index 2
///
/// PRG holds a `NOP; JMP self` spin loop at `$E000` (the fixed last bank) with
/// the reset vector pointing at it, so `StepFrame` renders while the CPU idles.
fn build_mmc2_rom() -> Vec<u8> {
    const PRG_BYTES: usize = 32 * 1024;
    const CHR_BYTES: usize = 8 * 1024;

    let mut rom = vec![0_u8; 16 + PRG_BYTES + CHR_BYTES];
    rom[0] = 0x4E; // N
    rom[1] = 0x45; // E
    rom[2] = 0x53; // S
    rom[3] = 0x1A;
    rom[4] = 2; // PRG: 2 x 16KB = 32KB
    rom[5] = 1; // CHR: 1 x 8KB = two 4KB banks
    rom[6] = (9 & 0x0F) << 4; // mapper 9 low nibble
    rom[7] = 9 & 0xF0; // mapper 9 high nibble

    let prg = 16;
    // Spin loop at $E000 (fixed last 8KB bank, PRG offset 24576): NOP; JMP $E000.
    let e000 = prg + 3 * PRG_8K;
    rom[e000] = 0xEA; // NOP
    rom[e000 + 1] = 0x4C; // JMP abs
    rom[e000 + 2] = 0x00;
    rom[e000 + 3] = 0xE0;
    // Reset vector at CPU $FFFC/$FFFD -> $E000.
    rom[prg + 0x7FFC] = 0x00;
    rom[prg + 0x7FFD] = 0xE0;

    // CHR: tile $01 pattern lives at bank-relative offset $10.
    let chr = 16 + PRG_BYTES;
    // Bank 0, tile $01 -> plane0 all set, plane1 clear => color index 1.
    for i in 0..8 {
        rom[chr + 0x10 + i] = 0xFF;
    }
    // Bank 1, tile $01 -> plane0 clear, plane1 all set => color index 2.
    for i in 0..8 {
        rom[chr + CHR_4K + 0x18 + i] = 0xFF;
    }

    rom
}

fn write_ppu_data(core: &mut NesCore, addr: u16, data: &[u8]) {
    core.write_cpu_bus(0x2006, (addr >> 8) as u8);
    core.write_cpu_bus(0x2006, addr as u8);
    for &byte in data {
        core.write_cpu_bus(0x2007, byte);
    }
}

/// Renders one frame of an MMC2 ROM whose background row 0 is `row0`, returning
/// the RGB of the pixel at (x=80, y=0). Column 10 (x=80) is far enough past the
/// top-left trigger tile and the CHR-swap deferral to reflect the latched bank.
fn render_row0_pixel_80(row0: &[u8; 32]) -> [u8; 3] {
    let mut core = NesCore::new();
    core.load_ines_rom(&build_mmc2_rom()).unwrap();

    // Low-half CHR banks: latch0==FD -> bank 1, latch0==FE -> bank 0.
    core.write_cpu_bus(0xB000, 1); // FD low bank
    core.write_cpu_bus(0xC000, 0); // FE low bank
    core.write_cpu_bus(0x2000, 0x00); // PPUCTRL: BG pattern table $0000, no NMI

    // Palette: index 1 = white ($30), index 2 = red ($16); distinct RGB.
    write_ppu_data(&mut core, 0x3F00, &[0x0F, 0x30, 0x16, 0x00]);
    // Background nametable row 0.
    write_ppu_data(&mut core, 0x2000, row0);

    // Reset scroll after $2006-based setup.
    core.write_cpu_bus(0x2005, 0x00);
    core.write_cpu_bus(0x2005, 0x00);
    core.write_cpu_bus(0x2001, 0x0A); // show background (+ leftmost 8px)

    core.execute(Command::StepFrame).unwrap();

    let frame = core.framebuffer_rgba();
    // Pixel (x=80, y=0).
    let idx = 80 * 4;
    [frame[idx], frame[idx + 1], frame[idx + 2]]
}

#[test]
fn mmc2_chr_latch_rebanks_background_through_ppu_render_path() {
    // No trigger tile in row 0: latch0 stays at its power-on FE state, so tile
    // $01 renders from the FE bank (color index 1).
    let mut no_trigger = [0x01_u8; 32];
    no_trigger[0] = 0x01;
    let baseline = render_row0_pixel_80(&no_trigger);

    // Same row, but the top-left tile is $FD: fetching it during rendering
    // flips latch0 -> FD via the PPU CHR-fetch hook, so tile $01 later on the
    // same row renders from the FD bank (color index 2).
    let mut with_trigger = [0x01_u8; 32];
    with_trigger[0] = 0xFD;
    let latched = render_row0_pixel_80(&with_trigger);

    assert_ne!(
        baseline, latched,
        "the FD tile fetch should re-bank the low CHR half through the PPU hook \
         (baseline={baseline:?}, latched={latched:?})"
    );
}
