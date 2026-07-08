use nes_core::mapper::Mmc3;

#[test]
fn mmc3_prg_mode_swaps_fixed_and_switchable_windows() {
    let mut m = Mmc3::new(8, 8);

    // Default mode: slot0=reg6(0), slot1=reg7(1), slot2=last-1(6), slot3=last(7).
    assert_eq!(m.read_prg(0x8000), 0);
    assert_eq!(m.read_prg(0xA000), 1);
    assert_eq!(m.read_prg(0xC000), 6);
    assert_eq!(m.read_prg(0xE000), 7);

    // Select register 6 and write bank value 4.
    m.write_prg(0x8000, 0x06);
    m.write_prg(0x8001, 0x04);
    assert_eq!(m.read_prg(0x8000), 4);

    // Toggle PRG mode (bit 6) while keeping register index 6 selected.
    m.write_prg(0x8000, 0x46);
    assert_eq!(m.read_prg(0x8000), 6);
    assert_eq!(m.read_prg(0xC000), 4);
}

#[test]
fn mmc3_chr_inversion_reorders_1k_and_2k_windows() {
    let mut m = Mmc3::new(8, 16);

    // Program CHR bank registers.
    m.write_prg(0x8000, 0x00);
    m.write_prg(0x8001, 0x02); // reg0 (2KB bank starts at 2)
    m.write_prg(0x8000, 0x01);
    m.write_prg(0x8001, 0x04); // reg1 (2KB bank starts at 4)
    m.write_prg(0x8000, 0x02);
    m.write_prg(0x8001, 0x06); // reg2
    m.write_prg(0x8000, 0x03);
    m.write_prg(0x8001, 0x07); // reg3
    m.write_prg(0x8000, 0x04);
    m.write_prg(0x8001, 0x01); // reg4
    m.write_prg(0x8000, 0x05);
    m.write_prg(0x8001, 0x03); // reg5

    let chr = m.chr_window();
    assert_eq!(chr[0x0000], 2);
    assert_eq!(chr[0x0400], 3);
    assert_eq!(chr[0x0800], 4);
    assert_eq!(chr[0x0C00], 5);
    assert_eq!(chr[0x1000], 6);
    assert_eq!(chr[0x1400], 7);
    assert_eq!(chr[0x1800], 1);
    assert_eq!(chr[0x1C00], 3);

    // Inverted mode swaps the 2KB and 1KB groups.
    m.write_prg(0x8000, 0x80);
    let inverted = m.chr_window();
    assert_eq!(inverted[0x0000], 6);
    assert_eq!(inverted[0x0400], 7);
    assert_eq!(inverted[0x0800], 1);
    assert_eq!(inverted[0x0C00], 3);
    assert_eq!(inverted[0x1000], 2);
    assert_eq!(inverted[0x1400], 3);
    assert_eq!(inverted[0x1800], 4);
    assert_eq!(inverted[0x1C00], 5);
}

// --- Scanline IRQ (A12 rising-edge model) ---------------------------------

/// One full PPU scanline is 341 dots (0..=340).
const DOTS_PER_SCANLINE: u16 = 341;

/// PPUCTRL for the common Tecmo-style layout: 8x8 sprites, background pattern
/// table $0000 (bit 4 clear), sprite pattern table $1000 (bit 3 set).
const CTRL_BG_LOW_SPR_HIGH: u8 = 0x08;

/// Drives every PPU dot of one scanline through the mapper's IRQ hook.
fn run_scanline(m: &mut Mmc3, scanline: u16, ctrl: u8) {
    for dot in 0..DOTS_PER_SCANLINE {
        m.on_ppu_dot(scanline, dot, true, ctrl);
    }
}

/// Counts scanline-counter clocks in a scanline using only the public API.
///
/// With latch 0 and IRQs enabled, every counter clock immediately asserts the
/// IRQ line, so we can count clocks by driving each dot and acknowledging the
/// pending flag after each one.
fn count_clocks_in_scanline(m: &mut Mmc3, scanline: u16, ctrl: u8) -> usize {
    let mut clocks = 0;
    for dot in 0..DOTS_PER_SCANLINE {
        m.on_ppu_dot(scanline, dot, true, ctrl);
        if m.irq_pending() {
            clocks += 1;
            m.write_prg(0xE000, 0x00); // acknowledge
            m.write_prg(0xE001, 0x00); // re-enable
        }
    }
    clocks
}

/// Number of counter clocks in one steady-state rendering scanline for a given
/// PPUCTRL configuration.
fn clocks_per_scanline(ctrl: u8) -> usize {
    let mut m = Mmc3::new(8, 8);
    m.write_prg(0xC000, 0x00); // latch 0 -> each clock fires the IRQ
    m.write_prg(0xC001, 0x00); // request reload
    m.write_prg(0xE001, 0x00); // enable IRQ
    // Settle the A12 filter (and consume the initial reload) before measuring.
    for scanline in 0..3 {
        count_clocks_in_scanline(&mut m, scanline, ctrl);
    }
    count_clocks_in_scanline(&mut m, 3, ctrl)
}

#[test]
fn mmc3_irq_common_config_fires_after_exactly_n_scanlines() {
    // BG=$0000, sprites=$1000, 8x8: exactly one clock per scanline. With latch
    // N (and a $C001 reload), the counter reloads to N on the first clock and
    // decrements to zero on the (N+1)-th, firing the IRQ.
    let latch = 4u8;
    let mut m = Mmc3::new(8, 8);
    m.write_prg(0xC000, latch);
    m.write_prg(0xC001, 0x00); // request reload
    m.write_prg(0xE001, 0x00); // enable IRQ

    // First scanline reloads; the next `latch` scanlines decrement to zero.
    for scanline in 0..u16::from(latch) {
        run_scanline(&mut m, scanline, CTRL_BG_LOW_SPR_HIGH);
        assert!(!m.irq_pending(), "should not fire before N+1 scanlines");
    }
    run_scanline(&mut m, u16::from(latch), CTRL_BG_LOW_SPR_HIGH);
    assert!(
        m.irq_pending(),
        "IRQ should fire on the (N+1)-th scanline clock"
    );

    m.write_prg(0xE000, 0x00); // disable + acknowledge
    assert!(!m.irq_pending());
}

#[test]
fn mmc3_irq_common_config_clocks_exactly_once_per_scanline() {
    assert_eq!(clocks_per_scanline(CTRL_BG_LOW_SPR_HIGH), 1);
}

#[test]
fn mmc3_irq_retimes_correctly_across_frames() {
    // After firing and acknowledging, the counter must re-arm and fire again on
    // schedule on subsequent frames.
    let latch = 2u8;
    let mut m = Mmc3::new(8, 8);
    m.write_prg(0xC000, latch);
    m.write_prg(0xC001, 0x00);
    m.write_prg(0xE001, 0x00);

    let mut fires = 0;
    for scanline in 0..30u16 {
        run_scanline(&mut m, scanline, CTRL_BG_LOW_SPR_HIGH);
        if m.irq_pending() {
            fires += 1;
            m.write_prg(0xE000, 0x00); // acknowledge
            m.write_prg(0xE001, 0x00); // re-enable
        }
    }
    // With latch 2 the IRQ fires roughly every (latch+1) scanlines after the
    // first reload; assert it retimes repeatedly rather than once.
    assert!(
        fires >= 8,
        "expected repeated IRQs across frames, got {fires}"
    );
}

#[test]
fn mmc3_irq_disabled_and_acknowledged_on_write_to_e000() {
    let mut m = Mmc3::new(8, 8);
    m.write_prg(0xC000, 0x01); // latch
    m.write_prg(0xC001, 0x00); // reload
    m.write_prg(0xE001, 0x00); // enable

    run_scanline(&mut m, 0, CTRL_BG_LOW_SPR_HIGH); // reload -> counter = 1
    run_scanline(&mut m, 1, CTRL_BG_LOW_SPR_HIGH); // 1 -> 0 -> IRQ
    assert!(m.irq_pending());

    m.write_prg(0xE000, 0x00); // disable and acknowledge
    assert!(!m.irq_pending());
}

#[test]
fn mmc3_irq_c001_forces_reload_mid_count() {
    let mut m = Mmc3::new(8, 8);
    m.write_prg(0xC000, 5); // latch
    m.write_prg(0xC001, 0x00); // reload
    m.write_prg(0xE001, 0x00); // enable

    run_scanline(&mut m, 0, CTRL_BG_LOW_SPR_HIGH); // reload to 5
    run_scanline(&mut m, 1, CTRL_BG_LOW_SPR_HIGH); // -> 4
    run_scanline(&mut m, 2, CTRL_BG_LOW_SPR_HIGH); // -> 3
    assert!(!m.irq_pending());

    m.write_prg(0xC001, 0x00); // force reload on next clock
    // It now takes a fresh reload (to 5) plus 5 decrements to fire again.
    for scanline in 3..8u16 {
        run_scanline(&mut m, scanline, CTRL_BG_LOW_SPR_HIGH);
        assert!(!m.irq_pending());
    }
    run_scanline(&mut m, 8, CTRL_BG_LOW_SPR_HIGH);
    assert!(m.irq_pending());
}

#[test]
fn mmc3_irq_not_triggered_if_rendering_disabled() {
    let mut m = Mmc3::new(8, 8);
    m.write_prg(0xC000, 0x01);
    m.write_prg(0xC001, 0x00);
    m.write_prg(0xE001, 0x00);

    for scanline in 0..5u16 {
        for dot in 0..DOTS_PER_SCANLINE {
            m.on_ppu_dot(scanline, dot, false, CTRL_BG_LOW_SPR_HIGH);
        }
    }
    assert!(!m.irq_pending());
}

#[test]
fn mmc3_irq_not_triggered_on_vblank_scanlines() {
    let mut m = Mmc3::new(8, 8);
    m.write_prg(0xC000, 0x01);
    m.write_prg(0xC001, 0x00);
    m.write_prg(0xE001, 0x00);

    for scanline in 240..=260u16 {
        run_scanline(&mut m, scanline, CTRL_BG_LOW_SPR_HIGH);
    }
    assert!(!m.irq_pending());
}

#[test]
fn mmc3_irq_shared_high_table_is_effectively_suppressed() {
    // BG and sprites both in $1000: the A12 filter suppresses the ~40 short
    // per-tile edges and the marginal horizontal-blank low window, so the
    // counter is never clocked (a shared pattern table cannot drive the split).
    let ctrl = 0x10 | 0x08; // BG=$1000, sprites=$1000
    assert_eq!(clocks_per_scanline(ctrl), 0);
}

#[test]
fn mmc3_irq_8x16_sprites_clock_once_per_scanline() {
    // 8x16 sprites, BG=$0000: one filtered edge per scanline (previously this
    // path was hardcoded to dot 260).
    let ctrl = 0x20; // 8x16 sprites, BG=$0000
    assert_eq!(clocks_per_scanline(ctrl), 1);
}

// --- PRG-RAM ($6000-$7FFF, $A001) -----------------------------------------

#[test]
fn mmc3_prg_ram_round_trips() {
    let mut m = Mmc3::new(8, 8);
    m.write_prg(0x6000, 0x12);
    m.write_prg(0x6ABC, 0x34);
    m.write_prg(0x7FFF, 0x56);
    assert_eq!(m.read_prg(0x6000), 0x12);
    assert_eq!(m.read_prg(0x6ABC), 0x34);
    assert_eq!(m.read_prg(0x7FFF), 0x56);
}

#[test]
fn mmc3_prg_ram_respects_write_protect_and_enable() {
    let mut m = Mmc3::new(8, 8);
    m.write_prg(0x6000, 0x11);

    // Enabled + write protected: writes ignored, reads still work.
    m.write_prg(0xA001, 0xC0); // bit7 enable, bit6 protect
    m.write_prg(0x6000, 0x22);
    assert_eq!(m.read_prg(0x6000), 0x11);

    // Disabled chip: reads float high, writes ignored.
    m.write_prg(0xA001, 0x00);
    assert_eq!(m.read_prg(0x6000), 0xFF);
    m.write_prg(0x6000, 0x33);

    // Re-enabled writable: prior contents preserved, writes land again.
    m.write_prg(0xA001, 0x80);
    assert_eq!(m.read_prg(0x6000), 0x11);
    m.write_prg(0x6000, 0x44);
    assert_eq!(m.read_prg(0x6000), 0x44);
}

#[test]
fn mmc3_prg_ram_does_not_alias_prg_rom() {
    let mut m = Mmc3::new(8, 8);
    // Writing PRG-RAM must not disturb banked PRG-ROM reads at $8000+.
    m.write_prg(0x6000, 0xEE);
    assert_eq!(m.read_prg(0x8000), 0);
    assert_eq!(m.read_prg(0xE000), 7);
}
