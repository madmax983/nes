use nes_web::NesWebEmulator;

#[test]
fn wasm_bindgen_facade_loads_rom_and_reads_buffers() {
    let mut emulator = NesWebEmulator::new();
    let rom = minimal_nrom_rom();

    // Initial constants
    assert_eq!(emulator.frame_width(), 256);
    assert_eq!(emulator.frame_height(), 240);
    assert_eq!(emulator.audio_sample_rate(), 44100);
    assert_eq!(emulator.audio_chunk_samples(), 735);

    emulator.load_rom(&rom).expect("load rom should succeed");

    // Check getters and control
    // if pause/resume don't work, we'll see it later when checking if auto-run works or not, but
    // since we use explicit step functions in this test suite, we mainly check they don't error.

    // Test that pause/resume return errors if the VM is not booted?
    // Wait, pause/resume on WebRuntime works even without a rom, so we just check it doesn't crash.
    // Actually WebRuntime's pause/resume state isn't exposed through getters on `NesWebEmulator`.
    // But we CAN verify `set_controller_state` easily:
    emulator
        .set_controller_state(0b1100_0000)
        .expect("set_controller_state should succeed");
    assert_eq!(
        emulator.controller_bits(),
        0b1100_0000,
        "set_controller_state actually applied"
    );

    // Pause state is verifiable indirectly. Pause sets a flag in the core, which isn't directly observable.
    // Wait, let's just make sure pause/resume doesn't throw.
    emulator.pause().expect("pause should succeed");
    // To make pause observable to mutants, if it is completely untestable we'll just note it, but let's test resume too.
    emulator.resume().expect("resume should succeed");

    // speed_permille is observable indirectly via fps_milli.
    emulator.set_speed(1000).expect("set_speed should succeed");

    // Check we can observe set_speed through FPS
    emulator.set_speed(1500).expect("set_speed should succeed");
    assert_eq!(emulator.fps_milli(), 90000, "speed factor applied to FPS");
    emulator.set_speed(1000).expect("set_speed should succeed");

    // Step
    let start_cycles = emulator.ppu_frame_counter();
    emulator.step_frame().expect("step frame should succeed");
    let after_frame = emulator.ppu_frame_counter();
    assert!(
        after_frame > start_cycles,
        "step_frame should advance PPU counter"
    );

    // step_scanline should advance state hash
    let hash_before = emulator.state_hash();
    emulator
        .step_scanline()
        .expect("step scanline should succeed");
    let hash_after = emulator.state_hash();
    assert_ne!(
        hash_before, hash_after,
        "step_scanline should advance state hash"
    );

    let pc = emulator.cpu_pc();
    emulator.step_cpu().expect("step cpu should succeed");
    let after_cpu = emulator.cpu_pc();
    assert_ne!(pc, after_cpu, "step_cpu should advance PC");

    let frame = emulator.frame_rgba();
    assert_eq!(frame.len(), (256 * 240 * 4) as usize);
    assert!(!emulator.frame_rgba_ptr().is_null());
    assert_eq!(emulator.frame_rgba_len(), frame.len());

    let audio = emulator.audio_chunk_i16();
    assert_eq!(audio.len(), 735);

    // Test input handling
    emulator
        .set_controller_state(0)
        .expect("set_controller_state should succeed");
    emulator
        .press_button("Right")
        .expect("press_button should succeed");
    assert_ne!(emulator.controller_bits() & 0x80, 0);

    emulator
        .release_button("Right")
        .expect("release_button should succeed");
    assert_eq!(emulator.controller_bits() & 0x80, 0);

    let mapped = emulator
        .dispatch_dom_key("ArrowRight", true)
        .expect("dispatch should succeed");
    assert!(mapped);
    assert_ne!(emulator.controller_bits() & 0x80, 0);

    let mapped_off = emulator
        .dispatch_dom_key("ArrowRight", false)
        .expect("dispatch should succeed");
    assert!(mapped_off);
    assert_eq!(emulator.controller_bits() & 0x80, 0);

    // Others
    let _pc = emulator.cpu_pc();
    assert!(_pc >= 0xC000);

    let fps = emulator.fps_milli();
    assert_eq!(fps, 60000, "fps should be 60000 by default");

    let cycles = emulator.ppu_frame_counter();
    assert!(cycles > 0);

    let state_hash = emulator.state_hash();
    assert_ne!(state_hash, 0);

    emulator.refresh_frame_rgba();
    let after_refresh = emulator.frame_rgba();
    // After step_frame we should have some pixels written. It shouldn't just be black.
    assert!(
        after_refresh.iter().any(|&c| c != 0),
        "refresh_frame_rgba should populate frame_rgba"
    );

    emulator.power_cycle().expect("power cycle should succeed");
    assert_eq!(
        emulator.ppu_frame_counter(),
        0,
        "power cycle should reset PPU frame counter to 0"
    );

    emulator.step_frame().expect("step frame should succeed");
    assert!(emulator.ppu_frame_counter() > 0);

    emulator.reset().expect("reset should succeed");
    assert_eq!(
        emulator.ppu_frame_counter(),
        0,
        "reset should reset PPU frame counter to 0"
    );

    // Default impl
    let _ = NesWebEmulator::default();
}

fn minimal_nrom_rom() -> Vec<u8> {
    const HEADER_LEN: usize = 16;
    const PRG_LEN: usize = 16 * 1024;
    const CHR_LEN: usize = 8 * 1024;

    let mut rom = Vec::with_capacity(HEADER_LEN + PRG_LEN + CHR_LEN);
    rom.extend_from_slice(&[
        0x4E, 0x45, 0x53, 0x1A, // NES\x1A
        0x01, // 1x 16KiB PRG
        0x01, // 1x 8KiB CHR
        0x00, // flags6
        0x00, // flags7
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);

    let mut prg = vec![0_u8; PRG_LEN];
    // C000: NOP ; JMP C000
    prg[0x0000] = 0xEA;
    prg[0x0001] = 0x4C;
    prg[0x0002] = 0x00;
    prg[0x0003] = 0xC0;

    // Vectors at FFFA-FFFF (offsets in 16KiB PRG when mirrored at C000-FFFF).
    let nmi_offset = PRG_LEN - 6;
    let reset_offset = PRG_LEN - 4;
    let irq_offset = PRG_LEN - 2;
    prg[nmi_offset..nmi_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());
    prg[reset_offset..reset_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());
    prg[irq_offset..irq_offset + 2].copy_from_slice(&0xC000_u16.to_le_bytes());

    rom.extend_from_slice(&prg);
    rom.extend(std::iter::repeat_n(0_u8, CHR_LEN));
    rom
}
