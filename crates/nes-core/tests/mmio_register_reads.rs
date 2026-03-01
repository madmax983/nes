use nes_core::{Command, NesCore};

#[test]
fn ppu_status_read_clears_vblank_bit() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000

    for _ in 0..300 {
        core.execute(Command::StepScanline).unwrap();
        if core.read_memory(0x2002) & 0x80 != 0 {
            break;
        }
    }
    assert_ne!(
        core.read_memory(0x2002) & 0x80,
        0,
        "expected vblank before read"
    );

    let pc = core.cpu_pc();
    core.load_cpu_bytes(pc, &[0xAD, 0x02, 0x20]); // LDA $2002
    core.execute(Command::StepCpu).unwrap();

    assert_ne!(core.cpu_a() & 0x80, 0, "read should observe vblank set");
    assert_eq!(
        core.read_memory(0x2002) & 0x80,
        0,
        "PPUSTATUS read must clear vblank"
    );
}

#[test]
fn ppudata_reads_are_buffered() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(
        0xC000,
        &[
            0xAD, 0x07, 0x20, // LDA $2007
            0x8D, 0x10, 0x00, // STA $0010
            0xAD, 0x07, 0x20, // LDA $2007
            0x8D, 0x11, 0x00, // STA $0011
        ],
    );

    write_ppu_bytes(&mut core, 0x0000, &[0x12, 0x34]);
    set_ppu_addr(&mut core, 0x0000);

    for _ in 0..4 {
        core.execute(Command::StepCpu).unwrap();
    }

    assert_eq!(
        core.read_memory(0x0010),
        0x00,
        "first read should return buffer"
    );
    assert_eq!(
        core.read_memory(0x0011),
        0x12,
        "second read should return first byte"
    );
}

#[test]
fn controller_port_reads_shift_bits_when_strobe_low() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(
        0xC000,
        &[
            0xA9, 0x01, // LDA #$01
            0x8D, 0x16, 0x40, // STA $4016 (strobe high)
            0xA9, 0x00, // LDA #$00
            0x8D, 0x16, 0x40, // STA $4016 (strobe low, latch)
            0xAD, 0x16, 0x40, // LDA $4016
            0x8D, 0x20, 0x00, // STA $0020
            0xAD, 0x16, 0x40, // LDA $4016
            0x8D, 0x21, 0x00, // STA $0021
        ],
    );
    core.execute(Command::SetControllerState(0x81)).unwrap(); // A + Right

    for _ in 0..8 {
        core.execute(Command::StepCpu).unwrap();
    }

    assert_eq!(core.read_memory(0x0020) & 1, 1, "A bit should read first");
    assert_eq!(core.read_memory(0x0021) & 1, 0, "B bit should read second");
}

#[test]
fn controller2_port_reads_shift_bits_from_4017() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(
        0xC000,
        &[
            0xA9, 0x01, // LDA #$01
            0x8D, 0x16, 0x40, // STA $4016 (strobe high)
            0xA9, 0x00, // LDA #$00
            0x8D, 0x16, 0x40, // STA $4016 (strobe low, latch both)
            0xAD, 0x17, 0x40, // LDA $4017
            0x8D, 0x22, 0x00, // STA $0022
            0xAD, 0x17, 0x40, // LDA $4017
            0x8D, 0x23, 0x00, // STA $0023
        ],
    );
    core.execute(Command::SetController2State(0x02)).unwrap(); // B only

    for _ in 0..8 {
        core.execute(Command::StepCpu).unwrap();
    }

    assert_eq!(core.read_memory(0x0022) & 1, 0, "A bit should read first");
    assert_eq!(core.read_memory(0x0023) & 1, 1, "B bit should read second");
}

#[test]
fn ppustatus_write_is_ignored() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000

    for _ in 0..300 {
        core.execute(Command::StepScanline).unwrap();
        if core.read_memory(0x2002) & 0x80 != 0 {
            break;
        }
    }
    assert_ne!(core.read_memory(0x2002) & 0x80, 0);

    core.write_cpu_bus(0x2002, 0x00);

    assert_ne!(
        core.read_memory(0x2002) & 0x80,
        0,
        "PPUSTATUS writes must not clear vblank"
    );
}

#[test]
fn oamdata_read_returns_byte_at_oamaddr() {
    let mut core = NesCore::new();
    core.write_cpu_bus(0x2003, 0x10);
    core.write_cpu_bus(0x2004, 0xAB);
    core.write_cpu_bus(0x2003, 0x10);

    assert_eq!(core.read_memory(0x2004), 0xAB);
}

#[test]
fn sprite_zero_hit_sets_ppustatus_bit() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000
    core.write_cpu_bus(0x2001, 0x1E); // show bg/sprites with left column enabled

    write_ppu_bytes(&mut core, 0x3F00, &[0x0F, 0x30, 0x00, 0x00]);
    write_ppu_bytes(&mut core, 0x0010, &[0xFF; 8]); // bg tile #1 plane 0
    write_ppu_bytes(&mut core, 0x0018, &[0x00; 8]); // bg tile #1 plane 1
    write_ppu_bytes(&mut core, 0x2000, &[0x01]); // nametable top-left uses tile #1
    write_ppu_bytes(&mut core, 0x0020, &[0xFF; 8]); // sprite tile #2 plane 0
    write_ppu_bytes(&mut core, 0x0028, &[0x00; 8]); // sprite tile #2 plane 1

    set_oam_sprite(&mut core, 0, 0, 2, 0, 0);
    core.execute(Command::StepScanline).unwrap();

    assert_ne!(
        core.read_memory(0x2002) & 0x40,
        0,
        "sprite zero hit should set PPUSTATUS bit 6"
    );
}

#[test]
fn sprite_zero_hit_requires_background_rendering_enabled() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000
    core.write_cpu_bus(0x2001, 0x10); // sprites only (BG disabled)

    write_ppu_bytes(&mut core, 0x0020, &[0xFF; 8]); // sprite tile #2 plane 0
    write_ppu_bytes(&mut core, 0x0028, &[0x00; 8]); // sprite tile #2 plane 1
    set_oam_sprite(&mut core, 0, 0, 2, 0, 0);

    core.execute(Command::StepScanline).unwrap();

    assert_eq!(
        core.read_memory(0x2002) & 0x40,
        0,
        "sprite-0 hit should not fire when BG rendering is disabled"
    );
}

#[test]
fn sprite_zero_hit_requires_non_transparent_background_pixel() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000
    core.write_cpu_bus(0x2001, 0x1E); // show bg/sprites with left column enabled

    // Sprite tile #2 is opaque at all bits.
    write_ppu_bytes(&mut core, 0x0020, &[0xFF; 8]); // sprite tile #2 plane 0
    write_ppu_bytes(&mut core, 0x0028, &[0x00; 8]); // sprite tile #2 plane 1
    set_oam_sprite(&mut core, 0, 0, 2, 0, 0);

    // Background remains transparent at overlap because nametable tile 0 pattern is all zeros.
    core.execute(Command::StepScanline).unwrap();

    assert_eq!(
        core.read_memory(0x2002) & 0x40,
        0,
        "sprite-0 hit should not fire without a non-transparent BG pixel overlap"
    );
}

#[test]
fn sprite_overflow_sets_ppustatus_bit() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000
    core.write_cpu_bus(0x2001, 0x10); // sprites enabled

    for i in 0..9_u8 {
        set_oam_sprite(&mut core, i, 0, i, 0, i.saturating_mul(8));
    }

    core.execute(Command::StepScanline).unwrap();
    core.execute(Command::StepScanline).unwrap();

    assert_ne!(
        core.read_memory(0x2002) & 0x20,
        0,
        "more than 8 sprites on a scanline should set overflow bit"
    );
}

fn set_ppu_addr(core: &mut NesCore, addr: u16) {
    core.write_cpu_bus(0x2006, (addr >> 8) as u8);
    core.write_cpu_bus(0x2006, addr as u8);
}

fn write_ppu_bytes(core: &mut NesCore, addr: u16, bytes: &[u8]) {
    set_ppu_addr(core, addr);
    for &byte in bytes {
        core.write_cpu_bus(0x2007, byte);
    }
}

fn set_oam_sprite(core: &mut NesCore, sprite_index: u8, y: u8, tile: u8, attr: u8, x: u8) {
    let base = sprite_index.saturating_mul(4);
    core.write_cpu_bus(0x2003, base);
    core.write_cpu_bus(0x2004, y);
    core.write_cpu_bus(0x2004, tile);
    core.write_cpu_bus(0x2004, attr);
    core.write_cpu_bus(0x2004, x);
}
