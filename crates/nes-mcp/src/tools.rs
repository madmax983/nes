//! Defines the available tools for the Model Context Protocol (MCP) server.
//!
//! This module maintains the central registry of all capabilities the emulator
//! exposes to external AI models. When the MCP client connects, the daemon
//! reads this catalog and translates it into the JSON-RPC response for the
//! `tools/list` protocol initialization phase.

/// Describes a single tool that can be invoked via the MCP.
///
/// These definitions match the exact names and descriptions returned to the
/// MCP client during the `tools/list` initialization phase.
///
/// ## Examples
///
/// ```
/// use nes_mcp::tools::ToolDefinition;
///
/// let tool = ToolDefinition {
///     name: "echo",
///     description: "Echoes back the input",
/// };
/// assert_eq!(tool.name, "echo");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToolDefinition {
    /// The exact string identifier sent in the tool invocation.
    pub name: &'static str,
    /// A human-readable description for LLMs to understand the tool's purpose.
    pub description: &'static str,
}

#[cfg(not(feature = "nova"))]
const TOOL_CATALOG_LEN: usize = 30;
#[cfg(feature = "nova")]
const TOOL_CATALOG_LEN: usize = 33;

const TOOL_CATALOG: [ToolDefinition; TOOL_CATALOG_LEN] = [
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
    ToolDefinition {
        name: "run_macro",
        description: "Execute a macro script to control the emulator",
    },
    ToolDefinition {
        name: "assemble_6502_dsl",
        description: "Assemble 6502 DSL source and report metadata",
    },
    ToolDefinition {
        name: "load_6502_dsl",
        description: "Build and load an iNES NROM ROM from 6502 DSL source",
    },
    ToolDefinition {
        name: "export_6502_dsl_rom",
        description: "Build and export an iNES NROM ROM from 6502 DSL source",
    },
    ToolDefinition {
        name: "export_6502_dsl_rom_base64",
        description: "Build an iNES NROM ROM from 6502 DSL source and return base64",
    },
    #[cfg(feature = "nova")]
    ToolDefinition {
        name: "scanner_reset",
        description: "Reset the experimental memory scanner, clearing all candidates",
    },
    #[cfg(feature = "nova")]
    ToolDefinition {
        name: "scanner_scan",
        description: "Scan main NES RAM using a condition to filter for state variables",
    },
    #[cfg(feature = "nova")]
    ToolDefinition {
        name: "scanner_candidates",
        description: "List the currently filtered memory addresses from the scanner",
    },
];

/// Returns the static catalog of all registered MCP tools.
///
/// The MCP daemon uses this list during the `tools/list` initialization phase to
/// broadcast available capabilities to the connected LLM client. Every tool
/// registered here must have a corresponding implementation in `dispatch::dispatch_tool`
/// and a schema definition in `tool_input_schema`.
///
/// ## Examples
///
/// ```
/// use nes_mcp::tools::tool_catalog;
///
/// let tools = tool_catalog();
/// assert!(!tools.is_empty());
/// assert!(tools.iter().any(|t| t.name == "load_rom"));
/// ```
#[must_use]
pub fn tool_catalog() -> &'static [ToolDefinition] {
    &TOOL_CATALOG
}
