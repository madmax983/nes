use nes_core::{AUDIO_CHUNK_SAMPLES, Button, Command, NesCore};
use nes_mcp::{DispatchError, DispatchOutput, ToolParams, dispatch_tool, tool_catalog};

fn params(pairs: &[(&str, &str)]) -> ToolParams {
    let mut map = ToolParams::new();
    for (k, v) in pairs {
        map.insert((*k).to_owned(), (*v).to_owned());
    }
    map
}

fn sample_nrom16_ines() -> Vec<u8> {
    let mut rom = vec![0_u8; 16 + 16 * 1024];
    rom[0] = 0x4E;
    rom[1] = 0x45;
    rom[2] = 0x53;
    rom[3] = 0x1A;
    rom[4] = 1;
    rom[5] = 0;

    let prg_start = 16;
    rom[prg_start] = 0xA9;
    rom[prg_start + 1] = 0x42;
    rom[prg_start + 0x3FFC] = 0x00;
    rom[prg_start + 0x3FFD] = 0x80;
    rom
}

fn sample_uxrom3_ines() -> Vec<u8> {
    let mut rom = vec![0_u8; 16 + 3 * 16 * 1024];
    rom[0] = 0x4E;
    rom[1] = 0x45;
    rom[2] = 0x53;
    rom[3] = 0x1A;
    rom[4] = 3;
    rom[5] = 0;
    rom[6] = 0x20; // mapper 2 low nibble in flags6 high bits.

    let prg_start = 16;
    let bank_size = 16 * 1024;

    // First bank ($8000) has distinct immediate value.
    rom[prg_start] = 0xA9;
    rom[prg_start + 1] = 0x11;

    // Last bank ($C000) contains reset entry and different immediate value.
    let last_bank = prg_start + 2 * bank_size;
    rom[last_bank] = 0xA9;
    rom[last_bank + 1] = 0x99;
    rom[last_bank + 0x3FFC] = 0x00;
    rom[last_bank + 0x3FFD] = 0xC0;

    rom
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push_str(&format!("{byte:02X}"));
    }
    output
}

#[test]
fn every_catalog_tool_has_dispatch_path() {
    let mut core = NesCore::new();
    let rom_hex = hex_encode(&sample_nrom16_ines());
    for tool in tool_catalog() {
        let params = match tool.name {
            "load_rom" => {
                let mut map = ToolParams::new();
                map.insert("rom_hex".to_owned(), rom_hex.clone());
                map
            }
            "press_button" | "release_button" => params(&[("button", "A")]),
            "set_controller_state" => params(&[("bits", "0x5A")]),
            "set_speed" => params(&[("multiplier", "1.5")]),
            "read_memory" => params(&[("address", "0xC000")]),
            "set_breakpoint" | "clear_breakpoint" | "disassemble_at" => {
                params(&[("address", "0xC000")])
            }
            "save_state" | "load_state" => params(&[("slot", "surface")]),
            _ => ToolParams::new(),
        };

        let result = dispatch_tool(&mut core, tool.name, &params);
        match result {
            Ok(_) => {}
            Err(DispatchError::InvalidParams(_))
            | Err(DispatchError::Core(_))
            | Err(DispatchError::StateSlotNotFound(_))
            | Err(DispatchError::Internal(_)) => {}
            Err(DispatchError::UnsupportedTool(name)) => assert_eq!(name, tool.name),
            Err(DispatchError::UnknownTool(name)) => {
                panic!("catalog tool {name} has no dispatch mapping")
            }
        }
    }
}

#[test]
fn press_and_release_button_tools_match_direct_commands() {
    let mut via_core = NesCore::new();
    via_core.execute(Command::PressButton(Button::A)).unwrap();
    via_core.execute(Command::ReleaseButton(Button::A)).unwrap();

    let mut via_mcp = NesCore::new();
    dispatch_tool(&mut via_mcp, "press_button", &params(&[("button", "A")])).unwrap();
    dispatch_tool(&mut via_mcp, "release_button", &params(&[("button", "A")])).unwrap();

    assert_eq!(via_core.state_hash(), via_mcp.state_hash());
}

#[test]
fn read_registers_and_memory_tools_reflect_core_state() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xA9, 0x7F]);
    dispatch_tool(&mut core, "step_cpu", &ToolParams::new()).unwrap();

    let regs = dispatch_tool(&mut core, "read_registers", &ToolParams::new()).unwrap();
    let mem = dispatch_tool(&mut core, "read_memory", &params(&[("address", "0xC001")])).unwrap();

    match regs {
        DispatchOutput::Registers { a, pc, .. } => {
            assert_eq!(a, 0x7F);
            assert_eq!(pc, 0xC002);
        }
        other => panic!("unexpected registers output: {other:?}"),
    }

    match mem {
        DispatchOutput::Memory { value, .. } => assert_eq!(value, 0x7F),
        other => panic!("unexpected memory output: {other:?}"),
    }
}

#[test]
fn get_ppu_frame_counter_reflects_core_progress() {
    let mut core = NesCore::new();
    dispatch_tool(&mut core, "step_frame", &ToolParams::new()).unwrap();
    dispatch_tool(&mut core, "step_frame", &ToolParams::new()).unwrap();

    let output = dispatch_tool(&mut core, "get_ppu_frame_counter", &ToolParams::new()).unwrap();
    match output {
        DispatchOutput::PpuFrameCounter { frame_counter } => assert!(frame_counter >= 1),
        other => panic!("unexpected get_ppu_frame_counter output: {other:?}"),
    }
}

#[test]
fn get_frame_reports_full_rgba_payload_size() {
    let mut core = NesCore::new();
    let output = dispatch_tool(&mut core, "get_frame", &ToolParams::new()).unwrap();
    match output {
        DispatchOutput::Frame { bytes, .. } => assert_eq!(bytes, 256 * 240 * 4),
        other => panic!("unexpected get_frame output: {other:?}"),
    }
}

#[test]
fn get_audio_chunk_reports_expected_sample_count() {
    let mut core = NesCore::new();
    let output = dispatch_tool(&mut core, "get_audio_chunk", &ToolParams::new()).unwrap();
    match output {
        DispatchOutput::Audio { samples, .. } => assert_eq!(samples, AUDIO_CHUNK_SAMPLES),
        other => panic!("unexpected get_audio_chunk output: {other:?}"),
    }
}

#[test]
fn save_and_load_state_round_trip_restores_state_hash() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA]);
    dispatch_tool(&mut core, "step_cpu", &ToolParams::new()).unwrap();
    dispatch_tool(&mut core, "save_state", &params(&[("slot", "slot0")])).unwrap();
    let saved_hash = core.state_hash();

    dispatch_tool(&mut core, "step_frame", &ToolParams::new()).unwrap();
    assert_ne!(saved_hash, core.state_hash());

    dispatch_tool(&mut core, "load_state", &params(&[("slot", "slot0")])).unwrap();
    assert_eq!(saved_hash, core.state_hash());
}

#[test]
fn load_rom_tool_maps_program_into_core_execution_path() {
    let mut core = NesCore::new();
    let rom_hex = hex_encode(&sample_nrom16_ines());

    let output = dispatch_tool(&mut core, "load_rom", &params(&[("rom_hex", &rom_hex)])).unwrap();
    match output {
        DispatchOutput::RomLoaded {
            mapper_id,
            prg_rom_bytes,
            reset_pc,
        } => {
            assert_eq!(mapper_id, 0);
            assert_eq!(prg_rom_bytes, 16 * 1024);
            assert_eq!(reset_pc, 0x8000);
        }
        other => panic!("unexpected load_rom output: {other:?}"),
    }

    dispatch_tool(&mut core, "step_cpu", &ToolParams::new()).unwrap();

    let regs = dispatch_tool(&mut core, "read_registers", &ToolParams::new()).unwrap();
    match regs {
        DispatchOutput::Registers { a, pc, .. } => {
            assert_eq!(a, 0x42);
            assert_eq!(pc, 0x8002);
        }
        other => panic!("unexpected registers output: {other:?}"),
    }
}

#[test]
fn load_rom_tool_supports_uxrom_boot_mapping() {
    let mut core = NesCore::new();
    let rom_hex = hex_encode(&sample_uxrom3_ines());

    let output = dispatch_tool(&mut core, "load_rom", &params(&[("rom_hex", &rom_hex)])).unwrap();
    match output {
        DispatchOutput::RomLoaded {
            mapper_id,
            prg_rom_bytes,
            reset_pc,
        } => {
            assert_eq!(mapper_id, 2);
            assert_eq!(prg_rom_bytes, 3 * 16 * 1024);
            assert_eq!(reset_pc, 0xC000);
        }
        other => panic!("unexpected load_rom output: {other:?}"),
    }

    let low = dispatch_tool(&mut core, "read_memory", &params(&[("address", "0x8001")])).unwrap();
    match low {
        DispatchOutput::Memory { value, .. } => assert_eq!(value, 0x11),
        other => panic!("unexpected memory output: {other:?}"),
    }

    dispatch_tool(&mut core, "step_cpu", &ToolParams::new()).unwrap();
    let regs = dispatch_tool(&mut core, "read_registers", &ToolParams::new()).unwrap();
    match regs {
        DispatchOutput::Registers { a, pc, .. } => {
            assert_eq!(a, 0x99);
            assert_eq!(pc, 0xC002);
        }
        other => panic!("unexpected registers output: {other:?}"),
    }
}
