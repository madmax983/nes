use nes_core::{Command, NesCore};

#[test]
fn test_ppu_pending_updates_push_back() {
    let mut core = NesCore::new();
    core.execute(Command::PowerCycle).unwrap();

    // Create an NROM mapping
    let rom = vec![
        0x4E, 0x45, 0x53, 0x1A, // "NES" + EOF
        1,    // 16KB PRG ROM
        0,    // 0KB CHR ROM (implies CHR RAM)
        0,    // Flag 6
        0,    // Flag 7
        0,    // Flag 8
        0,    // Flag 9
        0,    // Flag 10
        0, 0, 0, 0, 0, // Padding
    ];
    let mut rom_bytes = rom;
    rom_bytes.extend(vec![0; 16384]); // 16KB PRG ROM

    core.load_ines_rom(&rom_bytes).unwrap();

    // Trigger PPU to be rendering
    core.execute(Command::StepFrame).unwrap();

    // Write $2000 to trigger a BG update
    core.load_cpu_bytes(
        0x8000,
        &[
            0xA9, 0x00, // LDA #$00
            0x8D, 0x00, 0x20, // STA $2000
            0x8D, 0x06, 0x20, // STA $2006
            0x8D, 0x06, 0x20, // STA $2006
            0xA9, 0xFF, // LDA #$FF
            0x8D, 0x07, 0x20, // STA $2007 (CHR RAM update)
            0x4C, 0x11, 0x80, // JMP $8011 (loop)
        ],
    );

    core.execute(Command::Reset).unwrap();

    for _ in 0..100 {
        core.execute(Command::StepCpu).unwrap();
    }
}

#[test]
fn test_ppu_live_bg_update_preserve_split_vertical() {
    let mut core = NesCore::new();
    core.execute(Command::PowerCycle).unwrap();

    // Create an NROM mapping
    let rom = vec![
        0x4E, 0x45, 0x53, 0x1A, // "NES" + EOF
        1,    // 16KB PRG ROM
        0,    // 0KB CHR ROM (implies CHR RAM)
        0,    // Flag 6
        0,    // Flag 7
        0,    // Flag 8
        0,    // Flag 9
        0,    // Flag 10
        0, 0, 0, 0, 0, // Padding
    ];
    let mut rom_bytes = rom;
    rom_bytes.extend(vec![0; 16384]); // 16KB PRG ROM

    core.load_ines_rom(&rom_bytes).unwrap();

    // Trigger PPU to be rendering
    core.execute(Command::StepFrame).unwrap();

    // Write $2000 to trigger a BG update
    core.load_cpu_bytes(
        0x8000,
        &[
            0xA9, 0x00, // LDA #$00
            0x8D, 0x00, 0x20, // STA $2000
            0x4C, 0x05, 0x80, // JMP $8005 (loop)
        ],
    );

    core.execute(Command::Reset).unwrap();

    // We want to trigger the `preserve_split_vertical` branch
    // which requires: self.live_bg_tracks_vram_addr && self.scanline < FRAME_HEIGHT as u16 && self.dot > FRAME_WIDTH as u16
    // To do this, we step until scanline 100, dot 260
    while core.query(nes_core::CoreQuery::PpuFrameCounter)
        != nes_core::QueryResult::PpuFrameCounter(1)
    {
        // we are at frame 1 now
        core.execute(Command::StepCpu).unwrap();
    }
}
