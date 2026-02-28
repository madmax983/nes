#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: &'static str,
    pub description: &'static str,
}

const TOOL_CATALOG: [ToolDefinition; 25] = [
    ToolDefinition {
        name: "load_rom",
        description: "Load an iNES ROM into the emulator core",
    },
    ToolDefinition {
        name: "set_controller_state",
        description: "Set the entire controller button bitfield",
    },
    ToolDefinition {
        name: "press_button",
        description: "Press a single controller button",
    },
    ToolDefinition {
        name: "release_button",
        description: "Release a single controller button",
    },
    ToolDefinition {
        name: "reset",
        description: "Reset the emulator state",
    },
    ToolDefinition {
        name: "power_cycle",
        description: "Power cycle the emulator",
    },
    ToolDefinition {
        name: "pause",
        description: "Pause execution",
    },
    ToolDefinition {
        name: "resume",
        description: "Resume execution",
    },
    ToolDefinition {
        name: "set_speed",
        description: "Set emulation speed multiplier",
    },
    ToolDefinition {
        name: "get_frame",
        description: "Get the latest video frame payload",
    },
    ToolDefinition {
        name: "capture_frame",
        description: "Capture the latest video frame to a PPM or BMP file path",
    },
    ToolDefinition {
        name: "get_audio_chunk",
        description: "Get the latest audio chunk payload",
    },
    ToolDefinition {
        name: "get_fps",
        description: "Read the measured emulation FPS",
    },
    ToolDefinition {
        name: "get_ppu_frame_counter",
        description: "Read the PPU frame counter",
    },
    ToolDefinition {
        name: "get_emulator_state",
        description: "Read high-level emulator state",
    },
    ToolDefinition {
        name: "read_memory",
        description: "Read memory at an address",
    },
    ToolDefinition {
        name: "read_registers",
        description: "Read CPU register state",
    },
    ToolDefinition {
        name: "disassemble_at",
        description: "Disassemble opcode stream at address",
    },
    ToolDefinition {
        name: "step_cpu",
        description: "Step one CPU cycle",
    },
    ToolDefinition {
        name: "step_scanline",
        description: "Step one scanline",
    },
    ToolDefinition {
        name: "step_frame",
        description: "Step one frame",
    },
    ToolDefinition {
        name: "set_breakpoint",
        description: "Set a debugger breakpoint",
    },
    ToolDefinition {
        name: "clear_breakpoint",
        description: "Clear a debugger breakpoint",
    },
    ToolDefinition {
        name: "save_state",
        description: "Serialize emulator state",
    },
    ToolDefinition {
        name: "load_state",
        description: "Load serialized emulator state",
    },
];

#[must_use]
pub fn tool_catalog() -> &'static [ToolDefinition] {
    &TOOL_CATALOG
}
