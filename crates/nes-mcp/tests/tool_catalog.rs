use nes_mcp::tool_catalog;

#[test]
fn catalog_contains_required_user_facing_tools() {
    let tools = tool_catalog();
    for name in [
        "load_rom",
        "set_controller_state",
        "press_button",
        "release_button",
        "reset",
        "power_cycle",
        "pause",
        "resume",
        "set_speed",
        "get_frame",
        "get_audio_chunk",
        "get_fps",
        "get_emulator_state",
        "read_memory",
        "read_registers",
        "disassemble_at",
        "step_cpu",
        "step_scanline",
        "step_frame",
        "set_breakpoint",
        "clear_breakpoint",
        "save_state",
        "load_state",
    ] {
        assert!(tools.iter().any(|t| t.name == name), "missing {name}");
    }
}
