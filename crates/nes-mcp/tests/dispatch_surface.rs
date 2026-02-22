use nes_core::{Button, Command, NesCore};
use nes_mcp::{DispatchError, DispatchOutput, ToolParams, dispatch_tool, tool_catalog};

fn params(pairs: &[(&str, &str)]) -> ToolParams {
    let mut map = ToolParams::new();
    for (k, v) in pairs {
        map.insert((*k).to_owned(), (*v).to_owned());
    }
    map
}

#[test]
fn every_catalog_tool_has_dispatch_path() {
    let mut core = NesCore::new();
    for tool in tool_catalog() {
        let params = match tool.name {
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
