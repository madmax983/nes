use base64::{Engine as _, engine::general_purpose};
use nes_core::NesCore;
use nes_mcp::dispatch_tool;
use std::collections::BTreeMap;

#[test]
fn test_decode_hex_nibble_mutants() {
    let mut core = NesCore::new();
    let mut params = BTreeMap::new();

    // Testing load_rom which parses hex string
    params.insert("rom_hex".to_owned(), "ABcf1900".to_owned());
    let output = dispatch_tool(&mut core, "load_rom", &params);

    if let Err(e) = output {
        let msg = e.to_string();
        assert!(
            !msg.contains("invalid hex digit"),
            "Hex parsing failed: {}",
            msg
        );
    } else {
        panic!("Expected error loading invalid ROM");
    }
}

#[test]
fn test_encode_base64_loop_mutant() {
    let mut core = NesCore::new();
    let mut params = BTreeMap::new();

    params.insert(
        "source".to_owned(),
        "reset:\nLDA #$00\nSTA $00\nLDA #$00\nSTA $00\nLDA #$00".to_owned(),
    );
    let output = dispatch_tool(&mut core, "export_6502_dsl_rom_base64", &params).unwrap();

    if let nes_mcp::DispatchOutput::DslRomExportedBase64 { rom_base64, .. } = output {
        let decoded = general_purpose::STANDARD
            .decode(&rom_base64)
            .expect("Invalid base64 produced");
        assert_eq!(&decoded[0..4], b"NES\x1A");
        assert_eq!(rom_base64.len() % 4, 0);
    } else {
        panic!("Expected DslRomExportedBase64");
    }

    // Ensure loop operators / remainder branches are hit:
    // Testing multiple byte lengths triggers all branches of the base64 loop padding
    let lengths = vec![
        "",
        "LDA #$00",
        "LDA #$00\nNOP",
        "LDA #$00\nNOP\nNOP",
        "LDA #$00\nNOP\nNOP\nNOP",
    ];
    for src in lengths {
        params.insert("source".to_owned(), format!("reset:\n{}", src));
        let output = dispatch_tool(&mut core, "export_6502_dsl_rom_base64", &params).unwrap();
        if let nes_mcp::DispatchOutput::DslRomExportedBase64 {
            rom_base64, bytes, ..
        } = output
        {
            let decoded = general_purpose::STANDARD
                .decode(&rom_base64)
                .expect("Invalid base64 produced");
            assert_eq!(decoded.len(), bytes);
        }
    }
}

#[test]
fn test_parse_speed_permille_mutants() {
    let mut core = NesCore::new();
    let mut params = BTreeMap::new();

    params.insert("multiplier".to_owned(), "0.01".to_owned());
    dispatch_tool(&mut core, "set_speed", &params).unwrap();

    params.insert("multiplier".to_owned(), "10.0".to_owned());
    dispatch_tool(&mut core, "set_speed", &params).unwrap();

    params.insert("multiplier".to_owned(), "65.536".to_owned());
    let res = dispatch_tool(&mut core, "set_speed", &params);
    assert!(res.is_err(), "should reject out of range multiplier");
}

#[test]
fn test_parse_player2_mutants() {
    let mut core = NesCore::new();
    let mut params = BTreeMap::new();

    params.insert("bits".to_owned(), "0".to_owned());
    params.insert("player".to_owned(), "1".to_owned());
    dispatch_tool(&mut core, "set_controller_state", &params).unwrap();

    params.insert("player".to_owned(), "2".to_owned());
    dispatch_tool(&mut core, "set_controller_state", &params).unwrap();
}

#[test]
fn test_parse_u64_mutants() {
    let mut core = NesCore::new();
    let mut params = std::collections::BTreeMap::new();
    params.insert("frame".to_owned(), "10".to_owned());
    let _output = dispatch_tool(&mut core, "get_frame", &params);
}

#[test]
fn test_parse_u16_mutants() {
    let mut core = NesCore::new();
    let mut params = std::collections::BTreeMap::new();

    params.insert("address".to_owned(), "10".to_owned());
    dispatch_tool(&mut core, "read_memory", &params).unwrap();

    params.insert("address".to_owned(), "1000".to_owned());
    dispatch_tool(&mut core, "read_memory", &params).unwrap();
}

#[test]
fn test_parse_button_mutants() {
    let mut core = NesCore::new();
    let mut params = BTreeMap::new();

    let buttons = vec!["A", "B", "Select", "Start", "Up", "Down", "Left", "Right"];
    for btn in buttons {
        params.insert("button".to_owned(), btn.to_owned());
        dispatch_tool(&mut core, "press_button", &params).unwrap();
    }
}

#[test]
fn test_parse_slot_mutants() {
    let mut core = NesCore::new();
    let mut params = std::collections::BTreeMap::new();

    params.insert("slot".to_owned(), "1".to_owned());
    dispatch_tool(&mut core, "save_state", &params).unwrap();
    dispatch_tool(&mut core, "load_state", &params).unwrap();

    let empty_params = std::collections::BTreeMap::new();
    dispatch_tool(&mut core, "save_state", &empty_params).unwrap();
    dispatch_tool(&mut core, "load_state", &empty_params).unwrap();
}

#[test]
fn test_parse_integer_mutants() {
    let mut core = NesCore::new();
    let mut params = std::collections::BTreeMap::new();

    params.insert("address".to_owned(), "0x1A".to_owned());
    dispatch_tool(&mut core, "read_memory", &params).unwrap();
}

#[test]
fn test_parse_rom_payload_mutants() {
    let mut core = NesCore::new();
    let mut params = std::collections::BTreeMap::new();

    params.insert("rom_hex".to_owned(), "00".to_owned());
    let _ = dispatch_tool(&mut core, "load_rom", &params);

    let mut params2 = std::collections::BTreeMap::new();
    params2.insert("rom_base64".to_owned(), "AA==".to_owned());
    let _ = dispatch_tool(&mut core, "load_rom", &params2);
}

#[test]
fn test_parse_dsl_options_mutants() {
    let mut core = NesCore::new();
    let mut params = std::collections::BTreeMap::new();

    params.insert("source".to_owned(), "reset:\nLDA #$00".to_owned());
    params.insert("mirroring".to_owned(), "horizontal".to_owned());
    let _ = dispatch_tool(&mut core, "export_6502_dsl_rom", &params);

    params.insert("mirroring".to_owned(), "vertical".to_owned());
    let _ = dispatch_tool(&mut core, "export_6502_dsl_rom", &params);
}

#[test]
fn test_parse_dsl_source_mutants() {
    let mut core = NesCore::new();
    let mut params = std::collections::BTreeMap::new();

    params.insert("source".to_owned(), "reset:\nLDA #$00\nSTA $00".to_owned());
    let output = dispatch_tool(&mut core, "assemble_6502_dsl", &params).unwrap();

    if let nes_mcp::DispatchOutput::DslAssembled { bytes_written, .. } = output {
        assert_eq!(bytes_written, 4);
    } else {
        panic!("Expected DslAssembled");
    }
}

#[test]
fn test_parse_u8_mutants() {
    let mut core = NesCore::new();
    let mut params = std::collections::BTreeMap::new();
    params.insert("bits".to_owned(), "255".to_owned());
    dispatch_tool(&mut core, "set_controller_state", &params).unwrap();

    params.insert("bits".to_owned(), "256".to_owned());
    assert!(dispatch_tool(&mut core, "set_controller_state", &params).is_err());
}

#[test]
fn test_parse_required_string_mutants() {
    let mut core = NesCore::new();
    let mut params = std::collections::BTreeMap::new();

    params.insert("macro_name".to_owned(), "dummy".to_owned());
    let _ = dispatch_tool(&mut core, "run_macro", &params);
}

#[test]
fn test_dispatch_all_tools_mutants() {
    let mut core = NesCore::new();
    let params = std::collections::BTreeMap::new();
    let tools = vec![
        "pause",
        "resume",
        "reset",
        "power_cycle",
        "step_cpu",
        "step_scanline",
        "step_frame",
        "get_fps",
        "get_ppu_frame_counter",
        "get_ppu_oam",
        "get_emulator_state",
        "read_registers",
        "get_frame",
        "capture_frame",
        "get_audio_chunk",
    ];

    for tool in tools {
        let output = dispatch_tool(&mut core, tool, &params);
        if let Err(nes_mcp::DispatchError::UnknownTool(name)) = output {
            panic!("Tool match arm missing for {}", name);
        }
    }
}
