use nes_core::{Command, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH, NesCore};

#[test]
fn framebuffer_geometry_matches_nes_resolution() {
    let core = NesCore::new();
    let frame = core.framebuffer_rgba();

    assert_eq!(FRAME_WIDTH, 256);
    assert_eq!(FRAME_HEIGHT, 240);
    assert_eq!(FRAME_RGBA_BYTES, FRAME_WIDTH * FRAME_HEIGHT * 4);
    assert_eq!(frame.len(), FRAME_RGBA_BYTES);
    assert!(frame.as_chunks::<4>().0.iter().all(|px| px[3] == 0xFF));
}

#[test]
fn framebuffer_reflects_ppu_background_pattern_data() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000
    core.write_cpu_bus(0x2001, 0x0A); // enable background rendering + leftmost 8px

    write_ppu_data(&mut core, 0x3F00, &[0x0F, 0x30, 0x00, 0x00]); // universal + palette 0 color 1

    // Tile 1 pattern data: plane 0 set, plane 1 clear => color index 1 for all pixels.
    write_ppu_data(&mut core, 0x0010, &[0xFF; 8]);
    write_ppu_data(&mut core, 0x0018, &[0x00; 8]);
    write_ppu_data(&mut core, 0x2000, &[0x01]); // top-left nametable tile uses pattern 1

    core.execute(Command::StepFrame).unwrap();
    let frame = core.framebuffer_rgba();
    let top_left = &frame[0..4];
    assert_eq!(top_left[3], 0xFF);
    assert_ne!(top_left[0], 0);
    assert_ne!(top_left[1], 0);
    assert_ne!(top_left[2], 0);
}

#[test]
fn mid_frame_scroll_write_changes_later_scanline_pixels_only() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000
    core.write_cpu_bus(0x2001, 0x0A); // enable background rendering + leftmost 8px

    // Palette 0: color1 and color2 are distinct.
    write_ppu_data(&mut core, 0x3F00, &[0x0F, 0x16, 0x2A, 0x00]);

    // Tile 1 -> color index 1 for all pixels.
    write_ppu_data(&mut core, 0x0010, &[0xFF; 8]);
    write_ppu_data(&mut core, 0x0018, &[0x00; 8]);
    // Tile 2 -> color index 2 for all pixels.
    write_ppu_data(&mut core, 0x0020, &[0x00; 8]);
    write_ppu_data(&mut core, 0x0028, &[0xFF; 8]);

    // Row 0 starts with tile 1, row 1 starts with tile 2.
    write_ppu_data(&mut core, 0x2000, &[0x01]);
    write_ppu_data(&mut core, 0x2020, &[0x02]);

    // Reset scroll after $2006-based data setup (mirrors real-game VBlank behavior
    // where games always set scroll via $2005 before rendering begins).
    core.write_cpu_bus(0x2005, 0x00);
    core.write_cpu_bus(0x2005, 0x00);

    core.execute(Command::StepScanline).unwrap(); // render y=0
    core.write_cpu_bus(0x2005, 0x00); // scroll X
    core.write_cpu_bus(0x2005, 0x08); // scroll Y
    core.execute(Command::StepScanline).unwrap(); // finish scanline 1
    core.execute(Command::StepScanline).unwrap(); // render scanline 2 with new scroll

    let frame = core.framebuffer_rgba();
    let row0 = pixel_rgb(&frame, 0, 0);
    let row2 = pixel_rgb(&frame, 0, 2);

    assert_ne!(row0, row2, "later scanline should see new scroll value");
}

#[test]
fn timed_sta_to_ppuscroll_does_not_retroactively_rewrite_earlier_pixels() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(
        0xC000,
        &[
            0x8D, 0x05, 0x20, // STA $2005
            0x4C, 0x00, 0xC0, // JMP $C000
        ],
    );
    core.write_cpu_bus(0x2001, 0x0A); // enable background rendering + leftmost 8px

    // Palette 0: color1 and color2 are distinct.
    write_ppu_data(&mut core, 0x3F00, &[0x0F, 0x16, 0x2A, 0x00]);

    // Tile 1 -> color index 1 for all pixels.
    write_ppu_data(&mut core, 0x0010, &[0xFF; 8]);
    write_ppu_data(&mut core, 0x0018, &[0x00; 8]);
    // Tile 2 -> color index 2 for all pixels.
    write_ppu_data(&mut core, 0x0020, &[0x00; 8]);
    write_ppu_data(&mut core, 0x0028, &[0xFF; 8]);

    // Top-left tile uses tile 1, the next tile uses tile 2.
    write_ppu_data(&mut core, 0x2000, &[0x01, 0x02]);

    // Start from a known scroll state before placing the PPU at the start of a visible scanline.
    core.write_cpu_bus(0x2005, 0x00);
    core.write_cpu_bus(0x2005, 0x00);

    let mut snapshot = core.save_state();
    snapshot.cpu.pc = 0xC000;
    snapshot.cpu.a = 0x08; // new scroll_x selects tile 2 at the left edge
    snapshot.scheduler.cpu_cycles = 0;
    snapshot.scheduler.ppu_cycles = 0;
    snapshot.scheduler.apu_cycles = 0;
    snapshot.ppu.cycle_in_frame = 0;
    snapshot.ppu.scanline = 0;
    snapshot.ppu.dot = 0;
    snapshot.ppu.frame_counter = 0;
    snapshot.ppu.odd_frame = false;
    snapshot.ppu.status = 0;
    snapshot.ppu.ctrl = 0;
    snapshot.ppu.mask = 0x0A;
    snapshot.ppu.write_toggle = false;
    snapshot.ppu.vram_addr = 0;
    snapshot.ppu.temp_addr = 0;
    snapshot.ppu.fine_x = 0;
    snapshot.ppu.scroll_x = 0;
    snapshot.ppu.scroll_y = 0;
    snapshot.ppu.render_scroll_x = 0;
    snapshot.ppu.render_scroll_y = 0;
    snapshot.ppu.render_ctrl = 0;
    snapshot.ppu.render_capture_valid = false;
    core.load_state(&snapshot);

    let before = core.framebuffer_rgba();
    core.execute(Command::StepCpu).unwrap();
    let frame = core.framebuffer_rgba();

    let before_pixel_0 = pixel_rgb(&before, 0, 0);
    let before_pixel_8 = pixel_rgb(&before, 8, 0);
    let before_pixel_9 = pixel_rgb(&before, 9, 0);
    let pixel_0 = pixel_rgb(&frame, 0, 0);
    let pixel_8 = pixel_rgb(&frame, 8, 0);
    let pixel_9 = pixel_rgb(&frame, 9, 0);

    assert_eq!(
        pixel_0, before_pixel_0,
        "pixels rendered before the STA write cycle should keep the old scroll"
    );
    assert_eq!(
        pixel_8, before_pixel_8,
        "the first nine pixels should still reflect the pre-write scroll"
    );
    assert_ne!(
        pixel_9, before_pixel_9,
        "pixels rendered after the STA write cycle should see the new scroll"
    );
}

fn write_ppu_data(core: &mut NesCore, addr: u16, data: &[u8]) {
    core.write_cpu_bus(0x2006, (addr >> 8) as u8);
    core.write_cpu_bus(0x2006, addr as u8);
    for &byte in data {
        core.write_cpu_bus(0x2007, byte);
    }
}

fn pixel_rgb(frame: &[u8], x: usize, y: usize) -> [u8; 3] {
    let idx = (y * FRAME_WIDTH + x) * 4;
    [frame[idx], frame[idx + 1], frame[idx + 2]]
}
