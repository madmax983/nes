use nes_core::{AUDIO_CHUNK_SAMPLES, Button, Command, CoreSnapshot, NesCore};
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

fn decode_base64(raw: &str) -> Result<Vec<u8>, String> {
    fn decode_char(ch: u8) -> Result<u8, String> {
        match ch {
            b'A'..=b'Z' => Ok(ch - b'A'),
            b'a'..=b'z' => Ok(ch - b'a' + 26),
            b'0'..=b'9' => Ok(ch - b'0' + 52),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(format!("invalid base64 character '{}'", ch as char)),
        }
    }

    if !raw.len().is_multiple_of(4) {
        return Err("base64 length must be multiple of 4".to_owned());
    }

    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);
    let mut i = 0usize;
    while i < bytes.len() {
        let c0 = bytes[i];
        let c1 = bytes[i + 1];
        let c2 = bytes[i + 2];
        let c3 = bytes[i + 3];
        i += 4;

        let v0 = decode_char(c0)?;
        let v1 = decode_char(c1)?;
        let pad2 = c2 == b'=';
        let pad3 = c3 == b'=';
        let v2 = if pad2 { 0 } else { decode_char(c2)? };
        let v3 = if pad3 { 0 } else { decode_char(c3)? };

        out.push((v0 << 2) | (v1 >> 4));
        if !pad2 {
            out.push(((v1 & 0x0F) << 4) | (v2 >> 2));
        }
        if !pad3 {
            out.push(((v2 & 0x03) << 6) | v3);
        }
    }
    Ok(out)
}

#[test]
fn every_catalog_tool_has_dispatch_path() {
    let mut core = NesCore::new();
    let rom_hex = hex_encode(&sample_nrom16_ines());
    let export_path =
        std::env::temp_dir().join(format!("nes_mcp_export_surface_{}.nes", std::process::id()));
    let export_path_str = export_path.to_string_lossy().to_string();
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
            "assemble_6502_dsl" | "load_6502_dsl" => {
                params(&[("source", ".bank 1\nRESET:\n  JMP RESET\n.reset RESET\n")])
            }
            "export_6502_dsl_rom" => params(&[
                ("source", ".bank 1\nRESET:\n  JMP RESET\n.reset RESET\n"),
                ("output_path", export_path_str.as_str()),
            ]),
            "export_6502_dsl_rom_base64" => {
                params(&[("source", ".bank 1\nRESET:\n  JMP RESET\n.reset RESET\n")])
            }
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
    let _ = std::fs::remove_file(export_path);
}

#[test]
fn press_and_release_button_tools_match_direct_commands() {
    let mut via_core = NesCore::new();
    via_core.execute(Command::PressButton(Button::A)).expect("valid test data");
    via_core.execute(Command::ReleaseButton(Button::A)).expect("valid test data");

    let mut via_mcp = NesCore::new();
    dispatch_tool(&mut via_mcp, "press_button", &params(&[("button", "A")])).expect("valid test data");
    dispatch_tool(&mut via_mcp, "release_button", &params(&[("button", "A")])).expect("valid test data");

    assert_eq!(via_core.state_hash(), via_mcp.state_hash());
}

#[test]
fn controller_tools_support_player2_argument() {
    let mut core = NesCore::new();
    let output = dispatch_tool(
        &mut core,
        "set_controller_state",
        &params(&[("bits", "0xA5"), ("player", "2")]),
    )
    .expect("valid test data");
    match output {
        DispatchOutput::ControllerState { controller_bits } => assert_eq!(controller_bits, 0xA5),
        other => panic!("unexpected set_controller_state output: {other:?}"),
    }
    assert_eq!(core.controller_bits(), 0x00);
    assert_eq!(core.controller2_bits(), 0xA5);

    dispatch_tool(
        &mut core,
        "press_button",
        &params(&[("button", "A"), ("player", "2")]),
    )
    .expect("valid test data");
    assert_eq!(
        core.controller2_bits() & Button::A.bit_mask(),
        Button::A.bit_mask()
    );

    dispatch_tool(
        &mut core,
        "release_button",
        &params(&[("button", "A"), ("player", "2")]),
    )
    .expect("valid test data");
    assert_eq!(core.controller2_bits() & Button::A.bit_mask(), 0);
}

#[test]
fn read_registers_and_memory_tools_reflect_core_state() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xA9, 0x7F]);
    dispatch_tool(&mut core, "step_cpu", &ToolParams::new()).expect("valid test data");

    let regs = dispatch_tool(&mut core, "read_registers", &ToolParams::new()).expect("valid test data");
    let mem = dispatch_tool(&mut core, "read_memory", &params(&[("address", "0xC001")])).expect("valid test data");

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
    dispatch_tool(&mut core, "step_frame", &ToolParams::new()).expect("valid test data");
    dispatch_tool(&mut core, "step_frame", &ToolParams::new()).expect("valid test data");

    let output = dispatch_tool(&mut core, "get_ppu_frame_counter", &ToolParams::new()).expect("valid test data");
    match output {
        DispatchOutput::PpuFrameCounter { frame_counter } => assert!(frame_counter >= 1),
        other => panic!("unexpected get_ppu_frame_counter output: {other:?}"),
    }
}

#[test]
fn get_fps_returns_fps_output_variant() {
    let mut core = NesCore::new();
    let output = dispatch_tool(&mut core, "get_fps", &ToolParams::new()).expect("valid test data");
    match output {
        DispatchOutput::Fps { fps_milli } => assert!(fps_milli > 0),
        other => panic!("unexpected get_fps output: {other:?}"),
    }
}

#[test]
fn get_frame_reports_full_rgba_payload_size() {
    let mut core = NesCore::new();
    let output = dispatch_tool(&mut core, "get_frame", &ToolParams::new()).expect("valid test data");
    match output {
        DispatchOutput::Frame { bytes, .. } => assert_eq!(bytes, 256 * 240 * 4),
        other => panic!("unexpected get_frame output: {other:?}"),
    }
}

#[test]
fn capture_frame_writes_ppm_file() {
    let mut core = NesCore::new();
    let path = std::env::temp_dir().join("nes_mcp_capture_frame_test.ppm");
    let path_str = path.to_string_lossy().to_string();

    let output = dispatch_tool(
        &mut core,
        "capture_frame",
        &params(&[("path", path_str.as_str())]),
    )
    .expect("valid test data");
    match output {
        DispatchOutput::FrameCaptured {
            path: out_path,
            bytes,
        } => {
            assert_eq!(out_path, path_str);
            assert_eq!(bytes, 256 * 240 * 4);
        }
        other => panic!("unexpected capture_frame output: {other:?}"),
    }

    let ppm = std::fs::read(&path).expect("capture_frame should write output file");
    assert!(ppm.starts_with(b"P6\n256 240\n255\n"));
    assert!(ppm.len() > 16);
    let _ = std::fs::remove_file(path);
}

#[test]
fn capture_frame_writes_bmp_file() {
    let mut core = NesCore::new();
    let path = std::env::temp_dir().join("nes_mcp_capture_frame_test.bmp");
    let path_str = path.to_string_lossy().to_string();

    let output = dispatch_tool(
        &mut core,
        "capture_frame",
        &params(&[("path", path_str.as_str())]),
    )
    .expect("valid test data");
    match output {
        DispatchOutput::FrameCaptured {
            path: out_path,
            bytes,
        } => {
            assert_eq!(out_path, path_str);
            assert_eq!(bytes, 256 * 240 * 4);
        }
        other => panic!("unexpected capture_frame output: {other:?}"),
    }

    let bmp = std::fs::read(&path).expect("capture_frame should write output file");
    assert!(bmp.starts_with(b"BM"));
    assert!(bmp.len() > 64);
    let _ = std::fs::remove_file(path);
}

#[test]
fn get_audio_chunk_reports_expected_sample_count() {
    let mut core = NesCore::new();
    let output = dispatch_tool(&mut core, "get_audio_chunk", &ToolParams::new()).expect("valid test data");
    match output {
        DispatchOutput::Audio { samples, .. } => assert_eq!(samples, AUDIO_CHUNK_SAMPLES),
        other => panic!("unexpected get_audio_chunk output: {other:?}"),
    }
}

#[test]
fn save_and_load_state_round_trip_restores_state_hash() {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA]);
    dispatch_tool(&mut core, "step_cpu", &ToolParams::new()).expect("valid test data");
    dispatch_tool(&mut core, "save_state", &params(&[("slot", "slot0")])).expect("valid test data");
    let saved_hash = core.state_hash();

    dispatch_tool(&mut core, "step_frame", &ToolParams::new()).expect("valid test data");
    assert_ne!(saved_hash, core.state_hash());

    dispatch_tool(&mut core, "load_state", &params(&[("slot", "slot0")])).expect("valid test data");
    assert_eq!(saved_hash, core.state_hash());
}

#[test]
fn core_snapshot_size_stays_bounded_for_dispatch_stack_safety() {
    // Guard against accidental large inline buffers in saved state objects.
    // Regressions here can overflow stack frames in desktop/tests on Windows.
    const MAX_SNAPSHOT_BYTES: usize = 128 * 1024;
    assert!(
        std::mem::size_of::<CoreSnapshot>() <= MAX_SNAPSHOT_BYTES,
        "CoreSnapshot grew to {} bytes (limit {} bytes)",
        std::mem::size_of::<CoreSnapshot>(),
        MAX_SNAPSHOT_BYTES
    );
}

#[test]
fn load_rom_tool_maps_program_into_core_execution_path() {
    let mut core = NesCore::new();
    let rom_hex = hex_encode(&sample_nrom16_ines());

    let output = dispatch_tool(&mut core, "load_rom", &params(&[("rom_hex", &rom_hex)])).expect("valid test data");
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

    dispatch_tool(&mut core, "step_cpu", &ToolParams::new()).expect("valid test data");

    let regs = dispatch_tool(&mut core, "read_registers", &ToolParams::new()).expect("valid test data");
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

    let output = dispatch_tool(&mut core, "load_rom", &params(&[("rom_hex", &rom_hex)])).expect("valid test data");
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

    let low = dispatch_tool(&mut core, "read_memory", &params(&[("address", "0x8001")])).expect("valid test data");
    match low {
        DispatchOutput::Memory { value, .. } => assert_eq!(value, 0x11),
        other => panic!("unexpected memory output: {other:?}"),
    }

    dispatch_tool(&mut core, "step_cpu", &ToolParams::new()).expect("valid test data");
    let regs = dispatch_tool(&mut core, "read_registers", &ToolParams::new()).expect("valid test data");
    match regs {
        DispatchOutput::Registers { a, pc, .. } => {
            assert_eq!(a, 0x99);
            assert_eq!(pc, 0xC002);
        }
        other => panic!("unexpected registers output: {other:?}"),
    }
}

#[test]
fn assemble_6502_dsl_reports_expected_metadata() {
    let mut core = NesCore::new();
    let output = dispatch_tool(
        &mut core,
        "assemble_6502_dsl",
        &params(&[(
            "source",
            ".bank 1\nRESET:\n  LDA #$42\n  JMP RESET\n.reset RESET\n",
        )]),
    )
    .expect("valid test data");

    match output {
        DispatchOutput::DslAssembled {
            bytes_written,
            label_count,
            reset_vector,
            ..
        } => {
            assert_eq!(bytes_written, 5);
            assert!(label_count >= 1);
            assert_eq!(reset_vector, 0xC000);
        }
        other => panic!("unexpected assemble_6502_dsl output: {other:?}"),
    }
}

#[test]
fn load_6502_dsl_builds_and_loads_into_core() {
    let mut core = NesCore::new();
    let output = dispatch_tool(
        &mut core,
        "load_6502_dsl",
        &params(&[(
            "source",
            ".bank 1\nRESET:\n  LDA #$7F\n  JMP RESET\n.reset RESET\n",
        )]),
    )
    .expect("valid test data");
    match output {
        DispatchOutput::DslRomLoaded {
            mapper_id,
            prg_rom_bytes,
            reset_pc,
            ..
        } => {
            assert_eq!(mapper_id, 0);
            assert_eq!(prg_rom_bytes, 16 * 1024);
            assert_eq!(reset_pc, 0xC000);
        }
        other => panic!("unexpected load_6502_dsl output: {other:?}"),
    }

    dispatch_tool(&mut core, "step_cpu", &ToolParams::new()).expect("valid test data");
    let regs = dispatch_tool(&mut core, "read_registers", &ToolParams::new()).expect("valid test data");
    match regs {
        DispatchOutput::Registers { a, pc, .. } => {
            assert_eq!(a, 0x7F);
            assert_eq!(pc, 0xC002);
        }
        other => panic!("unexpected registers output: {other:?}"),
    }
}

#[test]
fn export_6502_dsl_rom_writes_ines_file() {
    let mut core = NesCore::new();
    let path = std::env::temp_dir().join("nes_mcp_export_6502_dsl_rom_test.nes");
    let path_str = path.to_string_lossy().to_string();

    let output = dispatch_tool(
        &mut core,
        "export_6502_dsl_rom",
        &params(&[
            (
                "source",
                ".bank 1\nRESET:\n  LDA #$11\n  JMP RESET\n.reset RESET\n",
            ),
            ("output_path", path_str.as_str()),
        ]),
    )
    .expect("valid test data");

    match output {
        DispatchOutput::DslRomExported {
            path: out_path,
            bytes,
            mapper_id,
            prg_rom_bytes,
        } => {
            assert_eq!(out_path, path_str);
            assert!(bytes > 16);
            assert_eq!(mapper_id, 0);
            assert_eq!(prg_rom_bytes, 16 * 1024);
        }
        other => panic!("unexpected export_6502_dsl_rom output: {other:?}"),
    }

    let rom = std::fs::read(&path).expect("export_6502_dsl_rom should write output file");
    assert!(rom.starts_with(b"NES\x1A"));
    assert_eq!(rom[4], 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn export_6502_dsl_rom_base64_returns_ines_payload() {
    let mut core = NesCore::new();
    let output = dispatch_tool(
        &mut core,
        "export_6502_dsl_rom_base64",
        &params(&[(
            "source",
            ".bank 1\nRESET:\n  LDA #$22\n  JMP RESET\n.reset RESET\n",
        )]),
    )
    .expect("valid test data");

    match output {
        DispatchOutput::DslRomExportedBase64 {
            rom_base64,
            bytes,
            mapper_id,
            prg_rom_bytes,
        } => {
            assert_eq!(mapper_id, 0);
            assert_eq!(prg_rom_bytes, 16 * 1024);
            let rom = decode_base64(&rom_base64).expect("valid base64 rom payload");
            assert_eq!(bytes, rom.len());
            assert!(rom.starts_with(b"NES\x1A"));
            assert_eq!(rom[4], 1);
        }
        other => panic!("unexpected export_6502_dsl_rom_base64 output: {other:?}"),
    }
}
