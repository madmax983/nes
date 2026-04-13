use nes_core::{
    AUDIO_CHUNK_SAMPLES, AUDIO_SAMPLE_RATE, FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH,
};
use nes_web::WebRuntime;

#[cfg(target_arch = "wasm32")]
use nes_web::NesWebEmulator;

#[test]
fn runtime_reports_expected_geometry_and_audio_constants() {
    let mut runtime = WebRuntime::new();
    let frame = runtime.refresh_frame_rgba();
    assert_eq!(frame.len(), FRAME_RGBA_BYTES);
    assert_eq!(runtime.frame_rgba_len(), FRAME_RGBA_BYTES);
    assert_eq!(runtime.frame_width(), FRAME_WIDTH as u32);
    assert_eq!(runtime.frame_height(), FRAME_HEIGHT as u32);
    assert_eq!(runtime.audio_sample_rate(), AUDIO_SAMPLE_RATE);
    assert_eq!(runtime.audio_chunk_samples(), AUDIO_CHUNK_SAMPLES as u32);
    assert!(!runtime.frame_rgba_ptr().is_null());
}

#[test]
fn runtime_rejects_invalid_rom_and_unknown_button_names() {
    let mut runtime = WebRuntime::new();
    let rom_err = runtime
        .load_rom(&[0x00, 0x01, 0x02, 0x03])
        .expect_err("invalid ROM bytes should fail");
    assert!(rom_err.contains("failed to load rom"));

    let press_err = runtime
        .press_button("NotAButton")
        .expect_err("unknown button press should fail");
    assert!(press_err.contains("unknown button"));

    let release_err = runtime
        .release_button("NotAButton")
        .expect_err("unknown button release should fail");
    assert!(release_err.contains("unknown button"));
}

#[test]
fn runtime_commands_mutate_observable_state() {
    let mut runtime = WebRuntime::new();
    runtime
        .load_rom(&minimal_nrom_rom())
        .expect("load rom should succeed");

    let reset_pc = runtime.cpu_pc();
    assert_eq!(reset_pc, 0xC000);

    let hash_before_pause = runtime.state_hash();
    runtime.pause().expect("pause should succeed");
    let paused_hash = runtime.state_hash();
    assert_ne!(paused_hash, hash_before_pause);
    runtime.resume().expect("resume should succeed");
    let resumed_hash = runtime.state_hash();
    assert_eq!(resumed_hash, hash_before_pause);

    runtime
        .set_speed(1_500)
        .expect("set_speed should accept non-zero speed");
    assert_eq!(runtime.fps_milli(), 90_000);

    runtime
        .set_controller_state(0xA5)
        .expect("set_controller_state should set all controller bits");
    assert_eq!(runtime.controller_bits(), 0xA5);

    for (name, mask) in [
        ("A", 0x01_u8),
        ("B", 0x02_u8),
        ("Select", 0x04_u8),
        ("Start", 0x08_u8),
        ("Up", 0x10_u8),
        ("Down", 0x20_u8),
        ("Left", 0x40_u8),
        ("Right", 0x80_u8),
    ] {
        runtime
            .set_controller_state(0)
            .expect("set controller state should succeed");
        runtime
            .press_button(name)
            .expect("press_button should accept known button");
        assert_eq!(runtime.controller_bits() & mask, mask, "press for {name}");
        runtime
            .release_button(name)
            .expect("release_button should accept known button");
        assert_eq!(runtime.controller_bits() & mask, 0, "release for {name}");
    }

    let scanline_hash_before = runtime.state_hash();
    runtime
        .step_scanline()
        .expect("step_scanline should execute on loaded ROM");
    assert_ne!(runtime.state_hash(), scanline_hash_before);

    runtime.step_cpu().expect("step_cpu should execute");
    assert_ne!(runtime.cpu_pc(), reset_pc);

    let frame_before = runtime.ppu_frame_counter();
    runtime.step_frame().expect("step_frame should execute");
    assert!(runtime.ppu_frame_counter() > frame_before);

    runtime.reset().expect("reset should succeed");
    assert_eq!(runtime.cpu_pc(), reset_pc);
    assert_eq!(runtime.controller_bits(), 0);
    assert_eq!(runtime.fps_milli(), 90_000);

    runtime.power_cycle().expect("power_cycle should succeed");
    assert_eq!(runtime.cpu_pc(), reset_pc);
    assert_eq!(runtime.controller_bits(), 0);
    assert_eq!(runtime.fps_milli(), 60_000);
}

#[cfg(target_arch = "wasm32")]
#[test]
fn emulator_wrapper_surfaces_runtime_behavior_and_errors() {
    let mut emu = NesWebEmulator::new();

    let bad_rom_err = emu
        .load_rom(&[0x00, 0x01, 0x02, 0x03])
        .expect_err("invalid rom should produce JsValue error");
    assert!(
        bad_rom_err
            .as_string()
            .expect("error should contain string payload")
            .contains("failed to load rom")
    );

    let bad_button_err = emu
        .press_button("NotAButton")
        .expect_err("unknown button should produce JsValue error");
    assert!(
        bad_button_err
            .as_string()
            .expect("error should contain string payload")
            .contains("unknown button")
    );

    emu.load_rom(&minimal_nrom_rom())
        .expect("load rom should succeed");
    let reset_pc = emu.cpu_pc();
    assert_eq!(reset_pc, 0xC000);

    emu.set_speed(1_500).expect("set speed should succeed");
    assert_eq!(emu.fps_milli(), 90_000);

    emu.set_controller_state(0).expect("set controller state");
    emu.press_button("Right").expect("press right");
    assert_eq!(emu.controller_bits() & 0x80, 0x80);
    emu.release_button("Right").expect("release right");
    assert_eq!(emu.controller_bits() & 0x80, 0);

    let mapped = emu
        .dispatch_dom_key("ArrowRight", true)
        .expect("mapped key should succeed");
    assert!(mapped);
    assert_eq!(emu.controller_bits() & 0x80, 0x80);

    let unmapped = emu
        .dispatch_dom_key("KeyQ", true)
        .expect("unmapped key should not error");
    assert!(!unmapped);

    emu.step_scanline().expect("step scanline should succeed");
    emu.step_cpu().expect("step cpu should succeed");
    assert_ne!(emu.cpu_pc(), reset_pc);

    let frame_before = emu.ppu_frame_counter();
    emu.step_frame().expect("step frame should succeed");
    assert!(emu.ppu_frame_counter() > frame_before);

    let hash_before_pause = emu.state_hash();
    emu.pause().expect("pause should succeed");
    let paused_hash = emu.state_hash();
    assert_ne!(paused_hash, hash_before_pause);
    emu.resume().expect("resume should succeed");
    assert_eq!(emu.state_hash(), hash_before_pause);

    emu.refresh_frame_rgba();
    let frame = emu.frame_rgba();
    assert_eq!(frame.len(), FRAME_RGBA_BYTES);
    assert_eq!(emu.frame_rgba_len(), FRAME_RGBA_BYTES);
    assert!(!emu.frame_rgba_ptr().is_null());

    let audio = emu.audio_chunk_i16();
    assert_eq!(audio.len(), AUDIO_CHUNK_SAMPLES);
    assert_eq!(emu.audio_sample_rate(), AUDIO_SAMPLE_RATE);
    assert_eq!(emu.audio_chunk_samples(), AUDIO_CHUNK_SAMPLES as u32);

    emu.reset().expect("reset should succeed");
    assert_eq!(emu.cpu_pc(), reset_pc);
    assert_eq!(emu.controller_bits(), 0);
    assert_eq!(emu.fps_milli(), 90_000);

    emu.power_cycle().expect("power_cycle should succeed");
    assert_eq!(emu.cpu_pc(), reset_pc);
    assert_eq!(emu.controller_bits(), 0);
    assert_eq!(emu.fps_milli(), 60_000);
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

#[test]
fn execute_commands_propagate_properly() {
    let mut runtime = WebRuntime::new();

    // If we step frame it succeeds returning `Ok(())`. `execute` doesn't return `Result` error from `Core::execute` since that never fails for these, but let's test it returns `Ok(true)` for DOM keys mapped.
    let mapped = runtime.dispatch_dom_key("KeyZ", true).unwrap();
    assert!(mapped);
    let unmapped = runtime.dispatch_dom_key("KeyQ", true).unwrap();
    assert!(!unmapped);
}
