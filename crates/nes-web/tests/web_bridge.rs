use nes_web::bridge::map_dom_key_to_command;
use nes_web::runtime::WebRuntime;

#[test]
fn dom_key_maps_to_press_button_command() {
    let cmd = map_dom_key_to_command("KeyX", true).unwrap();
    assert_eq!(cmd.tool_name(), "press_button");
}

#[test]
fn bridge_command_tool_name_returns_unsupported() {
    let cmd = nes_web::bridge::BridgeCommand { core: nes_core::Command::StepFrame };
    assert_eq!(cmd.tool_name(), "unsupported");
}

#[test]
fn dom_key_maps_all_supported_keys() {
    assert_eq!(map_dom_key_to_command("KeyZ", true).unwrap().core, nes_core::Command::PressButton(nes_core::Button::A));
    assert_eq!(map_dom_key_to_command("KeyX", true).unwrap().core, nes_core::Command::PressButton(nes_core::Button::B));
    assert_eq!(map_dom_key_to_command("Enter", true).unwrap().core, nes_core::Command::PressButton(nes_core::Button::Start));
    assert_eq!(map_dom_key_to_command("ShiftRight", true).unwrap().core, nes_core::Command::PressButton(nes_core::Button::Select));
    assert_eq!(map_dom_key_to_command("ArrowUp", true).unwrap().core, nes_core::Command::PressButton(nes_core::Button::Up));
    assert_eq!(map_dom_key_to_command("ArrowDown", true).unwrap().core, nes_core::Command::PressButton(nes_core::Button::Down));
    assert_eq!(map_dom_key_to_command("ArrowLeft", true).unwrap().core, nes_core::Command::PressButton(nes_core::Button::Left));
    assert_eq!(map_dom_key_to_command("ArrowRight", true).unwrap().core, nes_core::Command::PressButton(nes_core::Button::Right));
}

#[test]
fn release_button_command_maps_to_tool_name() {
    let cmd = map_dom_key_to_command("KeyZ", false).unwrap();
    assert_eq!(cmd.tool_name(), "release_button");
}

#[test]
fn runtime_loads_minimal_rom_and_produces_video_audio_buffers() {
    let rom = minimal_nrom_rom();
    let mut runtime = WebRuntime::new();
    runtime.load_rom(&rom).expect("load rom should succeed");

    for _ in 0..8 {
        runtime.step_frame().expect("step frame should succeed");
    }

    let frame = runtime.frame_rgba();
    assert_eq!(
        frame.len(),
        (runtime.frame_width() * runtime.frame_height() * 4) as usize
    );

    let audio = runtime.audio_chunk_i16();
    assert_eq!(audio.len(), runtime.audio_chunk_samples() as usize);
}

#[test]
fn runtime_dispatch_dom_key_updates_controller_state() {
    let rom = minimal_nrom_rom();
    let mut runtime = WebRuntime::new();
    runtime.load_rom(&rom).expect("load rom should succeed");

    let mapped = runtime
        .dispatch_dom_key("ArrowRight", true)
        .expect("dispatch should not fail");
    assert!(mapped);
    assert_ne!(runtime.controller_bits() & 0x80, 0);

    runtime
        .dispatch_dom_key("ArrowRight", false)
        .expect("dispatch should not fail");
    assert_eq!(runtime.controller_bits() & 0x80, 0);

    let unmapped = runtime
        .dispatch_dom_key("KeyQ", true)
        .expect("unmapped key should not error");
    assert!(!unmapped);
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
