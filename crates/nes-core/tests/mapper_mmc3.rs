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

#[test]
fn mmc3_irq_counter_reloads_and_raises_pending_flag() {
    let mut m = Mmc3::new(8, 8);
    m.write_prg(0xC000, 0x03); // latch
    m.write_prg(0xC001, 0x00); // request reload
    m.write_prg(0xE001, 0x00); // enable IRQ

    for scanline in 0..3 {
        m.on_ppu_dot(scanline, 260, true);
        assert!(!m.irq_pending());
    }

    m.on_ppu_dot(3, 260, true);
    assert!(m.irq_pending());

    m.write_prg(0xE000, 0x00); // disable + acknowledge
    assert!(!m.irq_pending());
}
