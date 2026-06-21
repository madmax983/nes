//! Maps MCP tool invocations to NES core execution and output queries.
//!
//! This module forms the central routing layer between the raw JSON-RPC string
//! maps provided by the MCP client and the strongly typed `nes-core` APIs. It
//! parses incoming `ToolParams`, validates them against the expected schema,
//! and translates emulator state back into the `DispatchOutput` enum for serialization.

use core::fmt;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::sync::{Mutex, OnceLock};

use nes_core::{
    Button, Command, CoreQuery, CoreSnapshot, FRAME_HEIGHT, FRAME_WIDTH, NesCore, QueryResult,
};
use nes_dsl::{Mirroring, RomBuildOptions};

use crate::output::{
    audio_chunk, frame_chunk, latest_output_metadata, publish_audio_with, publish_frame_with,
};

/// Type alias for key-value pair strings passed as arguments to an MCP tool.
///
/// This serves as a dynamic map of strings that the dispatch engine parses
/// into strict types according to the requirements of the requested tool.
///
/// ## Examples
///
/// ```
/// use nes_mcp::ToolParams;
///
/// let mut params = ToolParams::new();
/// params.insert("button".to_owned(), "A".to_owned());
/// params.insert("player".to_owned(), "1".to_owned());
/// ```
pub type ToolParams = BTreeMap<String, String>;

/// Represents the strongly typed response from a successful tool invocation.
///
/// Converts the internal emulator state or query results into a structured format
/// that the MCP server can format as JSON.
///
/// ## Examples
///
/// ```
/// use nes_mcp::DispatchOutput;
///
/// let ack = DispatchOutput::Ack;
/// let regs = DispatchOutput::Registers {
///     pc: 0x8000,
///     a: 0,
///     x: 0,
///     y: 0,
///     sp: 0xFD,
///     status: 0x24,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutput {
    /// Action completed successfully with no return data.
    Ack,
    /// Result of a single CPU step.
    CpuStep {
        /// Trace output of the instruction executed, if any.
        trace: Option<String>,
        /// The new total CPU cycle count.
        cpu_cycles: u64,
    },
    /// Result returning a cycle count.
    CycleCount {
        /// The new total CPU cycle count.
        cpu_cycles: u64,
    },
    /// Result returning the current state of controller inputs.
    ControllerState {
        /// Current state of the controller.
        controller_bits: u8,
    },
    /// Result returning high-level emulator metrics.
    EmulatorState {
        /// True if execution is paused.
        paused: bool,
        /// Current speed setting in permille.
        speed_permille: u16,
        /// Current state of the controller.
        controller_bits: u8,
    },
    /// Result returning CPU registers.
    Registers {
        /// Program Counter.
        pc: u16,
        /// Accumulator.
        a: u8,
        /// X index register.
        x: u8,
        /// Y index register.
        y: u8,
        /// Stack Pointer.
        sp: u8,
        /// Status flags.
        status: u8,
    },
    /// Result of memory inspection.
    Memory {
        /// The address that was inspected.
        address: u16,
        /// The value retrieved at the address.
        value: u8,
    },
    /// Result returning emulator framerate.
    Fps {
        /// Milliseconds of emulation time.
        fps_milli: u32,
    },
    /// Result returning PPU cycles.
    PpuFrameCounter {
        /// The total number of rendered frames.
        frame_counter: u64,
    },
    /// Result returning PPU OAM (Object Attribute Memory) data.
    PpuOam {
        /// The 256 bytes of OAM memory.
        oam_bytes: Vec<u8>,
    },
    /// Result referencing a published video frame.
    Frame {
        /// The sequence identifier for the frame payload.
        seq: u64,
        /// Size of the frame payload in bytes.
        bytes: usize,
    },
    /// Result of rendering a frame out to disk.
    FrameCaptured {
        /// Path the frame was written to.
        path: String,
        /// Size of the payload written.
        bytes: usize,
    },
    /// Result referencing a published audio block.
    Audio {
        /// Sequence identifier.
        seq: u64,
        /// Count of individual samples.
        samples: usize,
    },
    /// Save-state related result.
    StateSlot {
        /// Identifier for the saved state slot.
        slot: String,
    },
    /// Loaded cart configuration.
    RomLoaded {
        /// Inferred iNES mapper id.
        mapper_id: u8,
        /// Total PRG ROM byte size.
        prg_rom_bytes: usize,
        /// The PC reset vector.
        reset_pc: u16,
    },
    /// Result of executing a script of input sequences.
    MacroExecuted {
        /// Number of PPU frames processed.
        frames_elapsed: u64,
        /// Final controller bitfield.
        final_controller_bits: u8,
    },
    /// Statistics of assembled 6502 code.
    DslAssembled {
        /// Length of code bytes.
        bytes_written: usize,
        /// Total recognized labels.
        label_count: usize,
        /// Parsed NMI vector.
        nmi_vector: u16,
        /// Parsed Reset vector.
        reset_vector: u16,
        /// Parsed IRQ vector.
        irq_vector: u16,
    },
    /// Cart metadata after building from a 6502 text block.
    DslRomLoaded {
        /// Inferred iNES mapper id.
        mapper_id: u8,
        /// Total PRG ROM byte size.
        prg_rom_bytes: usize,
        /// The PC reset vector.
        reset_pc: u16,
        /// The resulting `.nes` bytes size.
        rom_bytes: usize,
    },
    /// Path metrics for exported 6502 script build.
    DslRomExported {
        /// Output disk path.
        path: String,
        /// Output file size in bytes.
        bytes: usize,
        /// Inferred iNES mapper id.
        mapper_id: u8,
        /// Total PRG ROM byte size.
        prg_rom_bytes: usize,
    },
    /// Memory metrics for exported 6502 script build.
    DslRomExportedBase64 {
        /// The output encoded payload.
        rom_base64: String,
        /// Real file size in bytes.
        bytes: usize,
        /// Inferred iNES mapper id.
        mapper_id: u8,
        /// Total PRG ROM byte size.
        prg_rom_bytes: usize,
    },
}

/// Errors raised when parsing or executing MCP tools.
///
/// This error acts as the primary failure boundary. If a requested tool does
/// not exist, has malformed inputs, or the underlying [`nes_core::NesCore`] throws
/// an internal exception, this enum safely propagates the error back to the
/// MCP JSON-RPC router as a structured response message.
///
/// ## Examples
///
/// ```
/// use nes_mcp::DispatchError;
///
/// // The client attempted to call a non-existent tool.
/// let err = DispatchError::UnknownTool("deploy_skynet".to_owned());
/// assert_eq!(err.to_string(), "unknown tool: deploy_skynet");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    /// The specified tool name was not recognized.
    UnknownTool(String),
    /// The specified tool name exists but is disabled or unimplemented.
    UnsupportedTool(String),
    /// Required parameters for the tool were missing or invalid.
    InvalidParams(String),
    /// An attempt to load from a save slot that does not exist.
    StateSlotNotFound(String),
    /// Subsystem error originating inside the NES emulator.
    Core(String),
    /// Panic, unexpected bounds checking, or IO subsystem error.
    Internal(String),
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTool(name) => write!(f, "unknown tool: {name}"),
            Self::UnsupportedTool(name) => write!(f, "unsupported tool: {name}"),
            Self::InvalidParams(msg) => write!(f, "invalid params: {msg}"),
            Self::StateSlotNotFound(slot) => write!(f, "state slot not found: {slot}"),
            Self::Core(msg) => write!(f, "core command failed: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for DispatchError {}

fn saved_states() -> &'static Mutex<HashMap<String, CoreSnapshot>> {
    static STATE: OnceLock<Mutex<HashMap<String, CoreSnapshot>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Dispatches an MCP tool call to the provided [`NesCore`] instance.
///
/// This function is the primary translation boundary between dynamic string maps
/// and strict type-checked emulator API interactions. It exists to decouple the
/// raw JSON/string payloads received by the MCP server from the internal execution
/// methods of the emulator core.
///
/// ## Errors
///
/// Returns a [`DispatchError`] if:
/// - The `tool_name` is unrecognized or unsupported.
/// - Required parameters are missing or invalid in `params`.
/// - The underlying [`NesCore`] encounters a runtime error.
///
/// ## Examples
///
/// ```
/// use nes_core::NesCore;
/// use nes_mcp::{dispatch_tool, DispatchOutput, ToolParams};
///
/// let mut core = NesCore::new();
/// let mut params = ToolParams::new();
/// let result = dispatch_tool(&mut core, "pause", &params).unwrap();
/// assert_eq!(result, DispatchOutput::Ack);
/// ```
pub fn dispatch_tool(
    core: &mut NesCore,
    tool_name: &str,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    match tool_name {
        "pause" => execute_command(core, Command::Pause).map(|_| DispatchOutput::Ack),
        "resume" => execute_command(core, Command::Resume).map(|_| DispatchOutput::Ack),
        "reset" => execute_command(core, Command::Reset).map(|_| DispatchOutput::Ack),
        "power_cycle" => execute_command(core, Command::PowerCycle).map(|_| DispatchOutput::Ack),
        "step_cpu" => handle_step_cpu(core),
        "step_scanline" => handle_step_scanline(core),
        "step_frame" => handle_step_frame(core),
        "set_controller_state" => handle_set_controller_state(core, params),
        "press_button" => handle_press_button(core, params),
        "release_button" => handle_release_button(core, params),
        "set_speed" => handle_set_speed(core, params),
        "get_fps" => handle_get_fps(core),
        "get_ppu_frame_counter" => handle_get_ppu_frame_counter(core),
        "get_ppu_oam" => handle_get_ppu_oam(core),
        "get_emulator_state" => handle_get_emulator_state(core),
        "read_registers" => handle_read_registers(core),
        "read_memory" => handle_read_memory(core, params),
        "get_frame" => handle_get_frame(core, params),
        "capture_frame" => handle_capture_frame(core, params),
        "get_audio_chunk" => handle_get_audio_chunk(core, params),
        "save_state" => handle_save_state(core, params),
        "load_state" => handle_load_state(core, params),
        "load_rom" => handle_load_rom(core, params),
        "run_macro" => handle_run_macro(core, params),
        "assemble_6502_dsl" => handle_assemble_6502_dsl(params),
        "load_6502_dsl" => handle_load_6502_dsl(core, params),
        "export_6502_dsl_rom" => handle_export_6502_dsl_rom(params),
        "export_6502_dsl_rom_base64" => handle_export_6502_dsl_rom_base64(params),
        "disassemble_at" | "set_breakpoint" | "clear_breakpoint" => {
            Err(DispatchError::UnsupportedTool(tool_name.to_owned()))
        }
        _ => Err(DispatchError::UnknownTool(tool_name.to_owned())),
    }
}

fn handle_step_cpu(core: &mut NesCore) -> Result<DispatchOutput, DispatchError> {
    execute_command(core, Command::StepCpu)?;
    Ok(DispatchOutput::CpuStep {
        trace: core.last_cpu_trace().map(ToOwned::to_owned),
        cpu_cycles: core.total_cycles(),
    })
}

fn handle_step_scanline(core: &mut NesCore) -> Result<DispatchOutput, DispatchError> {
    execute_command(core, Command::StepScanline)?;
    Ok(DispatchOutput::CycleCount {
        cpu_cycles: core.total_cycles(),
    })
}

fn handle_step_frame(core: &mut NesCore) -> Result<DispatchOutput, DispatchError> {
    execute_command(core, Command::StepFrame)?;
    Ok(DispatchOutput::CycleCount {
        cpu_cycles: core.total_cycles(),
    })
}

fn handle_set_speed(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let speed = parse_speed_permille(params)?;
    execute_command(core, Command::SetSpeed(speed))?;
    Ok(DispatchOutput::EmulatorState {
        paused: core.is_paused(),
        speed_permille: core.speed_permille(),
        controller_bits: core.controller_bits(),
    })
}

fn handle_get_fps(core: &NesCore) -> Result<DispatchOutput, DispatchError> {
    match core.query(CoreQuery::FpsMilli) {
        QueryResult::FpsMilli(fps_milli) => Ok(DispatchOutput::Fps { fps_milli }),
        _ => Err(DispatchError::Internal(
            "unexpected core query result for get_fps".to_owned(),
        )),
    }
}

fn handle_get_ppu_oam(core: &NesCore) -> Result<DispatchOutput, DispatchError> {
    let mut oam_bytes = Vec::with_capacity(256);
    for i in 0..=255 {
        oam_bytes.push(core.ppu_oam_byte(i));
    }
    Ok(DispatchOutput::PpuOam { oam_bytes })
}

fn handle_get_ppu_frame_counter(core: &NesCore) -> Result<DispatchOutput, DispatchError> {
    match core.query(CoreQuery::PpuFrameCounter) {
        QueryResult::PpuFrameCounter(frame_counter) => {
            Ok(DispatchOutput::PpuFrameCounter { frame_counter })
        }
        _ => Err(DispatchError::Internal(
            "unexpected core query result for get_ppu_frame_counter".to_owned(),
        )),
    }
}

fn handle_get_emulator_state(core: &NesCore) -> Result<DispatchOutput, DispatchError> {
    match core.query(CoreQuery::EmulatorState) {
        QueryResult::EmulatorState(state) => Ok(DispatchOutput::EmulatorState {
            paused: state.paused,
            speed_permille: state.speed_permille,
            controller_bits: state.controller_bits,
        }),
        _ => Err(DispatchError::Internal(
            "unexpected core query result for get_emulator_state".to_owned(),
        )),
    }
}

fn handle_read_registers(core: &NesCore) -> Result<DispatchOutput, DispatchError> {
    match core.query(CoreQuery::Registers) {
        QueryResult::Registers(regs) => Ok(DispatchOutput::Registers {
            pc: regs.pc,
            a: regs.a,
            x: regs.x,
            y: regs.y,
            sp: regs.sp,
            status: regs.status,
        }),
        _ => Err(DispatchError::Internal(
            "unexpected core query result for read_registers".to_owned(),
        )),
    }
}

fn handle_read_memory(
    core: &NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let address = parse_u16(params, "address")?;
    match core.query(CoreQuery::Memory(address)) {
        QueryResult::Memory(value) => Ok(DispatchOutput::Memory { address, value }),
        _ => Err(DispatchError::Internal(
            "unexpected core query result for read_memory".to_owned(),
        )),
    }
}

fn handle_get_frame(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    sync_frame_output(core);
    let default_seq = latest_output_metadata().frame_seq.saturating_add(1);
    let requested_seq = parse_u64(params, "seq").unwrap_or(default_seq);
    let chunk = frame_chunk(requested_seq)
        .ok_or_else(|| DispatchError::Internal("frame chunk missing".to_owned()))?;
    Ok(DispatchOutput::Frame {
        seq: chunk.seq,
        bytes: chunk.rgba.len(),
    })
}

fn handle_get_audio_chunk(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    sync_audio_output(core);
    let default_seq = latest_output_metadata().audio_seq.saturating_add(1);
    let requested_seq = parse_u64(params, "seq").unwrap_or(default_seq);
    let chunk = audio_chunk(requested_seq)
        .ok_or_else(|| DispatchError::Internal("audio chunk missing".to_owned()))?;
    Ok(DispatchOutput::Audio {
        seq: chunk.seq,
        samples: chunk.samples.len(),
    })
}

fn handle_run_macro(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let Some(script) = params.get("script") else {
        return Err(DispatchError::InvalidParams(
            "script must be provided".to_owned(),
        ));
    };
    let frames_elapsed = crate::macro_engine::execute_macro_script(core, script, None)
        .map_err(DispatchError::InvalidParams)?;
    Ok(DispatchOutput::MacroExecuted {
        frames_elapsed,
        final_controller_bits: core.controller_bits(),
    })
}

fn handle_assemble_6502_dsl(params: &ToolParams) -> Result<DispatchOutput, DispatchError> {
    let source = parse_dsl_source(params)?;
    let assembled = nes_dsl::assemble(source)
        .map_err(|err| DispatchError::InvalidParams(format!("dsl assembly failed: {err}")))?;
    Ok(DispatchOutput::DslAssembled {
        bytes_written: assembled.bytes.len(),
        label_count: assembled.labels.len(),
        nmi_vector: assembled.nmi_vector,
        reset_vector: assembled.reset_vector,
        irq_vector: assembled.irq_vector,
    })
}

fn handle_load_6502_dsl(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let source = parse_dsl_source(params)?;
    let options = parse_dsl_rom_options(params)?;
    let rom = nes_dsl::build_ines_nrom_rom(source, &options)
        .map_err(|err| DispatchError::InvalidParams(format!("dsl rom build failed: {err}")))?;
    let info = core
        .load_ines_rom(&rom)
        .map_err(|err| DispatchError::Core(err.to_string()))?;
    Ok(DispatchOutput::DslRomLoaded {
        mapper_id: info.mapper_id,
        prg_rom_bytes: info.prg_rom_bytes,
        reset_pc: info.reset_pc,
        rom_bytes: rom.len(),
    })
}

fn handle_export_6502_dsl_rom(params: &ToolParams) -> Result<DispatchOutput, DispatchError> {
    let source = parse_dsl_source(params)?;
    let options = parse_dsl_rom_options(params)?;
    let output_path = parse_required_string(params, "output_path")?;
    let rom = nes_dsl::build_ines_nrom_rom(source, &options)
        .map_err(|err| DispatchError::InvalidParams(format!("dsl rom build failed: {err}")))?;
    fs::write(&output_path, &rom).map_err(|err| {
        DispatchError::InvalidParams(format!(
            "unable to write output_path '{}': {err}",
            output_path
        ))
    })?;

    let prg_rom_bytes = rom
        .get(4)
        .map(|banks| usize::from(*banks) * 16 * 1024)
        .unwrap_or(0);
    Ok(DispatchOutput::DslRomExported {
        path: output_path.to_owned(),
        bytes: rom.len(),
        mapper_id: 0,
        prg_rom_bytes,
    })
}

fn handle_export_6502_dsl_rom_base64(params: &ToolParams) -> Result<DispatchOutput, DispatchError> {
    let source = parse_dsl_source(params)?;
    let options = parse_dsl_rom_options(params)?;
    let rom = nes_dsl::build_ines_nrom_rom(source, &options)
        .map_err(|err| DispatchError::InvalidParams(format!("dsl rom build failed: {err}")))?;
    let prg_rom_bytes = rom
        .get(4)
        .map(|banks| usize::from(*banks) * 16 * 1024)
        .unwrap_or(0);
    Ok(DispatchOutput::DslRomExportedBase64 {
        rom_base64: encode_base64(rom.as_slice()),
        bytes: rom.len(),
        mapper_id: 0,
        prg_rom_bytes,
    })
}

fn handle_set_controller_state(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let bits = parse_u8(params, "bits")?;
    let player2 = parse_player2(params)?;
    execute_command(
        core,
        if player2 {
            Command::SetController2State(bits)
        } else {
            Command::SetControllerState(bits)
        },
    )?;
    Ok(DispatchOutput::ControllerState {
        controller_bits: if player2 {
            core.controller2_bits()
        } else {
            core.controller_bits()
        },
    })
}

fn handle_press_button(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let button = parse_button(params)?;
    let player2 = parse_player2(params)?;
    execute_command(
        core,
        if player2 {
            Command::PressButton2(button)
        } else {
            Command::PressButton(button)
        },
    )?;
    Ok(DispatchOutput::ControllerState {
        controller_bits: if player2 {
            core.controller2_bits()
        } else {
            core.controller_bits()
        },
    })
}

fn handle_release_button(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let button = parse_button(params)?;
    let player2 = parse_player2(params)?;
    execute_command(
        core,
        if player2 {
            Command::ReleaseButton2(button)
        } else {
            Command::ReleaseButton(button)
        },
    )?;
    Ok(DispatchOutput::ControllerState {
        controller_bits: if player2 {
            core.controller2_bits()
        } else {
            core.controller_bits()
        },
    })
}

fn handle_capture_frame(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let Some(path) = params.get("path") else {
        return Err(DispatchError::InvalidParams(
            "path must be provided".to_owned(),
        ));
    };
    let rgba = core.framebuffer_rgba();
    write_frame_image(path, FRAME_WIDTH, FRAME_HEIGHT, &rgba)?;
    Ok(DispatchOutput::FrameCaptured {
        path: path.to_owned(),
        bytes: rgba.len(),
    })
}

fn handle_save_state(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let slot = parse_slot(params);
    let mut slots = saved_states()
        .lock()
        .map_err(|_| DispatchError::Internal("saved-state lock poisoned".to_owned()))?;
    slots.insert(slot.clone(), core.save_state());
    Ok(DispatchOutput::StateSlot { slot })
}

fn handle_load_state(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let slot = parse_slot(params);
    let snapshot = {
        let slots = saved_states()
            .lock()
            .map_err(|_| DispatchError::Internal("saved-state lock poisoned".to_owned()))?;
        slots.get(&slot).cloned()
    };
    let Some(snapshot) = snapshot else {
        return Err(DispatchError::StateSlotNotFound(slot));
    };

    core.load_state(&snapshot);
    Ok(DispatchOutput::StateSlot { slot })
}

fn handle_load_rom(
    core: &mut NesCore,
    params: &ToolParams,
) -> Result<DispatchOutput, DispatchError> {
    let rom_bytes = parse_rom_payload(params)?;
    let info = core
        .load_ines_rom(&rom_bytes)
        .map_err(|err| DispatchError::Core(err.to_string()))?;
    Ok(DispatchOutput::RomLoaded {
        mapper_id: info.mapper_id,
        prg_rom_bytes: info.prg_rom_bytes,
        reset_pc: info.reset_pc,
    })
}

fn execute_command(core: &mut NesCore, command: Command) -> Result<(), DispatchError> {
    core.execute(command)
        .map_err(|err| DispatchError::Core(err.to_string()))
}

fn sync_frame_output(core: &NesCore) {
    publish_frame_with(FRAME_WIDTH as u32, FRAME_HEIGHT as u32, |frame| {
        core.fill_framebuffer_rgba(frame)
    });
}

fn sync_audio_output(core: &mut NesCore) {
    publish_audio_with(nes_core::AUDIO_CHUNK_SAMPLES, |samples| {
        core.fill_audio_chunk_i16(samples)
    });
}

fn write_frame_image(
    path: &str,
    width: usize,
    height: usize,
    rgba: &[u8],
) -> Result<(), DispatchError> {
    let expected = width
        .checked_mul(height)
        .and_then(|px| px.checked_mul(4))
        .ok_or_else(|| DispatchError::Internal("frame size overflow".to_owned()))?;
    if rgba.len() != expected {
        return Err(DispatchError::Internal(
            "frame rgba length does not match dimensions".to_owned(),
        ));
    }

    let image = if path.to_ascii_lowercase().ends_with(".bmp") {
        nes_core::bmp::encode_bmp(width, height, rgba).map_err(DispatchError::Internal)?
    } else {
        nes_core::ppm::encode_ppm(width, height, rgba)
            .map_err(|e| DispatchError::Internal(e.to_string()))?
    };
    fs::write(path, image)
        .map_err(|err| DispatchError::InvalidParams(format!("unable to write '{path}': {err}")))
}

fn parse_button(params: &ToolParams) -> Result<Button, DispatchError> {
    let Some(button) = params.get("button").map(String::as_str) else {
        return Err(DispatchError::InvalidParams(
            "button must be provided".to_owned(),
        ));
    };

    match button {
        "A" => Ok(Button::A),
        "B" => Ok(Button::B),
        "Select" => Ok(Button::Select),
        "Start" => Ok(Button::Start),
        "Up" => Ok(Button::Up),
        "Down" => Ok(Button::Down),
        "Left" => Ok(Button::Left),
        "Right" => Ok(Button::Right),
        _ => Err(DispatchError::InvalidParams(format!(
            "unknown button '{button}'"
        ))),
    }
}

fn parse_u8(params: &ToolParams, key: &str) -> Result<u8, DispatchError> {
    let Some(raw) = params.get(key) else {
        return Err(DispatchError::InvalidParams(format!(
            "{key} must be provided"
        )));
    };
    let value = parse_integer(raw)?;
    u8::try_from(value)
        .map_err(|_| DispatchError::InvalidParams(format!("{key} must be an integer in [0, 255]")))
}

fn parse_u16(params: &ToolParams, key: &str) -> Result<u16, DispatchError> {
    let Some(raw) = params.get(key) else {
        return Err(DispatchError::InvalidParams(format!(
            "{key} must be provided"
        )));
    };
    let value = parse_integer(raw)?;
    u16::try_from(value).map_err(|_| {
        DispatchError::InvalidParams(format!("{key} must be an integer in [0, 65535]"))
    })
}

fn parse_u64(params: &ToolParams, key: &str) -> Option<u64> {
    let raw = params.get(key)?;
    parse_integer(raw).ok()
}

fn parse_speed_permille(params: &ToolParams) -> Result<u16, DispatchError> {
    let Some(raw) = params.get("multiplier") else {
        return Err(DispatchError::InvalidParams(
            "multiplier must be provided".to_owned(),
        ));
    };

    let multiplier: f64 = raw.parse().map_err(|_| {
        DispatchError::InvalidParams("multiplier must be a positive number".to_owned())
    })?;
    if !multiplier.is_finite() || multiplier <= 0.0 {
        return Err(DispatchError::InvalidParams(
            "multiplier must be a positive number".to_owned(),
        ));
    }

    let permille = (multiplier * 1_000.0).round();
    if !(1.0..=(u16::MAX as f64)).contains(&permille) {
        return Err(DispatchError::InvalidParams(
            "multiplier is out of supported range".to_owned(),
        ));
    }

    Ok(permille as u16)
}

fn parse_player2(params: &ToolParams) -> Result<bool, DispatchError> {
    let Some(raw) = params.get("player") else {
        return Ok(false);
    };
    let value = parse_integer(raw)?;
    match value {
        1 => Ok(false),
        2 => Ok(true),
        _ => Err(DispatchError::InvalidParams(
            "player must be 1 or 2".to_owned(),
        )),
    }
}

fn parse_slot(params: &ToolParams) -> String {
    params
        .get("slot")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| "default".to_owned())
}

fn parse_integer(raw: &str) -> Result<u64, DispatchError> {
    if let Some(stripped) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        u64::from_str_radix(stripped, 16)
            .map_err(|_| DispatchError::InvalidParams(format!("invalid integer literal '{raw}'")))
    } else {
        raw.parse::<u64>()
            .map_err(|_| DispatchError::InvalidParams(format!("invalid integer literal '{raw}'")))
    }
}

fn parse_rom_payload(params: &ToolParams) -> Result<Vec<u8>, DispatchError> {
    if let Some(path) = params.get("rom_path") {
        return fs::read(path).map_err(|err| {
            DispatchError::InvalidParams(format!("unable to read rom_path '{path}': {err}"))
        });
    }

    let Some(hex) = params.get("rom_hex") else {
        return Err(DispatchError::InvalidParams(
            "provide rom_hex or rom_path".to_owned(),
        ));
    };
    parse_hex_bytes(hex)
}

fn parse_dsl_source(params: &ToolParams) -> Result<&str, DispatchError> {
    let Some(source) = params.get("source").map(String::as_str) else {
        return Err(DispatchError::InvalidParams(
            "source must be provided".to_owned(),
        ));
    };
    if source.trim().is_empty() {
        return Err(DispatchError::InvalidParams(
            "source must not be empty".to_owned(),
        ));
    }
    Ok(source)
}

fn parse_dsl_rom_options(params: &ToolParams) -> Result<RomBuildOptions, DispatchError> {
    let mut options = RomBuildOptions::default();

    if let Some(raw) = params.get("mirroring") {
        options.mirroring = match raw.to_ascii_lowercase().as_str() {
            "horizontal" => Mirroring::Horizontal,
            "vertical" => Mirroring::Vertical,
            _ => {
                return Err(DispatchError::InvalidParams(
                    "mirroring must be 'horizontal' or 'vertical'".to_owned(),
                ));
            }
        };
    }

    if let Some(chr_hex) = params.get("chr_hex") {
        options.chr_rom = parse_hex_bytes(chr_hex)?;
    }

    Ok(options)
}

/// **⚡ Bolt Optimization:** Returns an `&str` reference from the `ToolParams` dictionary
/// instead of calling `.cloned()`. This eliminates unnecessary heap allocations
/// when tool parameters are just passed to other functions without requiring ownership.
fn parse_required_string<'a>(params: &'a ToolParams, key: &str) -> Result<&'a str, DispatchError> {
    let Some(value) = params.get(key) else {
        return Err(DispatchError::InvalidParams(format!(
            "{key} must be provided"
        )));
    };
    if value.trim().is_empty() {
        return Err(DispatchError::InvalidParams(format!(
            "{key} must not be empty"
        )));
    }
    Ok(value.as_str())
}

fn encode_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut i = 0usize;
    while i + 3 <= bytes.len() {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        let b2 = bytes[i + 2];
        i += 3;

        out.push(ALPHABET[usize::from(b0 >> 2)] as char);
        out.push(ALPHABET[usize::from(((b0 & 0x03) << 4) + (b1 >> 4))] as char);
        out.push(ALPHABET[usize::from(((b1 & 0x0F) << 2) + (b2 >> 6))] as char);
        out.push(ALPHABET[usize::from(b2 & 0x3F)] as char);
    }

    let rem = bytes.len() - i;
    if rem == 1 {
        let b0 = bytes[i];
        out.push(ALPHABET[usize::from(b0 >> 2)] as char);
        out.push(ALPHABET[usize::from((b0 & 0x03) << 4)] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        out.push(ALPHABET[usize::from(b0 >> 2)] as char);
        out.push(ALPHABET[usize::from(((b0 & 0x03) << 4) + (b1 >> 4))] as char);
        out.push(ALPHABET[usize::from((b1 & 0x0F) << 2)] as char);
        out.push('=');
    }

    out
}

fn parse_hex_bytes(raw: &str) -> Result<Vec<u8>, DispatchError> {
    let mut bytes = Vec::with_capacity(raw.len() / 2);
    let mut valid_bytes = raw
        .bytes()
        .enumerate()
        .filter(|&(_, b)| !b.is_ascii_whitespace() && b != b'_');

    while let Some((i1, b1)) = valid_bytes.next() {
        let Some((i2, b2)) = valid_bytes.next() else {
            return Err(DispatchError::InvalidParams(
                "rom_hex must have an even number of hex digits".to_owned(),
            ));
        };

        let hi = decode_hex_nibble(b1, i1)?;
        let lo = decode_hex_nibble(b2, i2)?;
        bytes.push((hi << 4) + lo);
    }
    Ok(bytes)
}

fn decode_hex_nibble(ch: u8, index: usize) -> Result<u8, DispatchError> {
    match ch {
        b'0'..=b'9' => Ok(ch - b'0'),
        b'a'..=b'f' => Ok(ch - b'a' + 10),
        b'A'..=b'F' => Ok(ch - b'A' + 10),
        _ => Err(DispatchError::InvalidParams(format!(
            "rom_hex has invalid hex digit '{}' at index {}",
            ch as char, index
        ))),
    }
}

#[cfg(test)]
mod tests {
    use nes_core::{FRAME_HEIGHT, FRAME_WIDTH, NesCore};

    use super::{
        Button, DispatchError, Mirroring, ToolParams, encode_base64, parse_button,
        parse_dsl_rom_options, parse_hex_bytes, parse_player2, parse_slot, parse_speed_permille,
        parse_u64, sync_audio_output, sync_frame_output,
    };
    use crate::output::{latest_output_metadata, reset_output_state_for_test};

    fn params(pairs: &[(&str, &str)]) -> ToolParams {
        let mut map = ToolParams::new();
        for (key, value) in pairs {
            map.insert((*key).to_owned(), (*value).to_owned());
        }
        map
    }

    #[test]
    fn parse_button_supports_all_button_names() {
        for (name, expected) in [
            ("A", Button::A),
            ("B", Button::B),
            ("Select", Button::Select),
            ("Start", Button::Start),
            ("Up", Button::Up),
            ("Down", Button::Down),
            ("Left", Button::Left),
            ("Right", Button::Right),
        ] {
            let button = parse_button(&params(&[("button", name)])).expect("valid button");
            assert_eq!(button, expected);
        }
    }

    #[test]
    fn parse_u64_handles_decimal_hex_missing_and_invalid_values() {
        assert_eq!(parse_u64(&ToolParams::new(), "frame"), None);
        assert_eq!(parse_u64(&params(&[("frame", "42")]), "frame"), Some(42));
        assert_eq!(parse_u64(&params(&[("frame", "0x2A")]), "frame"), Some(42));
        assert_eq!(
            parse_u64(&params(&[("frame", "not-a-number")]), "frame"),
            None
        );
    }

    #[test]
    fn parse_speed_permille_validates_and_converts_multiplier() {
        assert_eq!(
            parse_speed_permille(&params(&[("multiplier", "1.5")])).expect("valid multiplier"),
            1500
        );

        let bad_number =
            parse_speed_permille(&params(&[("multiplier", "abc")])).expect_err("invalid number");
        assert_eq!(
            bad_number.to_string(),
            "invalid params: multiplier must be a positive number"
        );

        let not_positive =
            parse_speed_permille(&params(&[("multiplier", "0")])).expect_err("non-positive");
        assert_eq!(
            not_positive.to_string(),
            "invalid params: multiplier must be a positive number"
        );

        let not_finite =
            parse_speed_permille(&params(&[("multiplier", "NaN")])).expect_err("not finite");
        assert_eq!(
            not_finite.to_string(),
            "invalid params: multiplier must be a positive number"
        );

        let out_of_range =
            parse_speed_permille(&params(&[("multiplier", "1000")])).expect_err("out of range");
        assert_eq!(
            out_of_range.to_string(),
            "invalid params: multiplier is out of supported range"
        );
    }

    #[test]
    fn parse_player2_maps_player_slot_values() {
        assert!(!parse_player2(&ToolParams::new()).expect("default player"));
        assert!(!parse_player2(&params(&[("player", "1")])).expect("player one"));
        assert!(parse_player2(&params(&[("player", "2")])).expect("player two"));

        let err = parse_player2(&params(&[("player", "3")])).expect_err("invalid player");
        assert!(matches!(err, DispatchError::InvalidParams(_)));
    }

    #[test]
    fn parse_slot_uses_default_when_omitted() {
        assert_eq!(parse_slot(&ToolParams::new()), "default");
        assert_eq!(parse_slot(&params(&[("slot", "quicksave")])), "quicksave");
    }

    #[test]
    fn parse_dsl_rom_options_supports_mirroring_and_chr_hex() {
        let defaults = parse_dsl_rom_options(&ToolParams::new()).expect("default options");
        assert_eq!(defaults.mirroring, Mirroring::Horizontal);
        assert_eq!(defaults.chr_rom.len(), 8 * 1024);

        let horizontal = parse_dsl_rom_options(&params(&[("mirroring", "horizontal")]))
            .expect("horizontal options");
        assert_eq!(horizontal.mirroring, Mirroring::Horizontal);

        let vertical =
            parse_dsl_rom_options(&params(&[("mirroring", "vertical"), ("chr_hex", "AA 55")]))
                .expect("vertical options");
        assert_eq!(vertical.mirroring, Mirroring::Vertical);
        assert_eq!(vertical.chr_rom, vec![0xAA, 0x55]);

        let err = parse_dsl_rom_options(&params(&[("mirroring", "diagonal")]))
            .expect_err("invalid mirroring");
        assert!(matches!(err, DispatchError::InvalidParams(_)));
    }

    #[test]
    fn encode_base64_matches_known_vectors() {
        assert_eq!(encode_base64(b""), "");
        assert_eq!(encode_base64(b"f"), "Zg==");
        assert_eq!(encode_base64(b"fo"), "Zm8=");
        assert_eq!(encode_base64(b"foo"), "Zm9v");
        assert_eq!(encode_base64(b"foob"), "Zm9vYg==");
        assert_eq!(encode_base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn parse_hex_bytes_supports_mixed_case_whitespace_and_underscores() {
        let parsed = parse_hex_bytes("de ad_BE ef").expect("valid mixed hex");
        assert_eq!(parsed, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn parse_hex_bytes_reports_shape_and_digit_errors() {
        let odd = parse_hex_bytes("ABC").expect_err("odd hex length");
        assert_eq!(
            odd.to_string(),
            "invalid params: rom_hex must have an even number of hex digits"
        );

        let bad = parse_hex_bytes("AAx0").expect_err("invalid hex digit");
        assert_eq!(
            bad.to_string(),
            "invalid params: rom_hex has invalid hex digit 'x' at index 2"
        );

        let bad_low_nibble =
            parse_hex_bytes("AA0x").expect_err("invalid low nibble index should be precise");
        assert_eq!(
            bad_low_nibble.to_string(),
            "invalid params: rom_hex has invalid hex digit 'x' at index 3"
        );
    }

    #[test]
    fn sync_outputs_publish_frame_and_audio_sequences() {
        let _guard = reset_output_state_for_test();
        let mut core = NesCore::new();
        let before = latest_output_metadata();

        sync_frame_output(&core);
        let after_frame = latest_output_metadata();
        assert!(after_frame.frame_seq > before.frame_seq);
        assert_eq!(after_frame.width, FRAME_WIDTH as u32);
        assert_eq!(after_frame.height, FRAME_HEIGHT as u32);

        sync_audio_output(&mut core);
        let after_audio = latest_output_metadata();
        assert!(after_audio.audio_seq > before.audio_seq);
    }

    #[test]
    fn handle_get_ppu_oam_returns_256_bytes() {
        let core = NesCore::new();
        let result = super::handle_get_ppu_oam(&core).expect("Should successfully retrieve OAM");
        match result {
            super::DispatchOutput::PpuOam { oam_bytes } => {
                assert_eq!(oam_bytes.len(), 256);
            }
            _ => panic!("Expected DispatchOutput::PpuOam"),
        }
    }
}
#[cfg(test)]
mod havoc_fuzz_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(10000))]
        #[test]
        fn havoc_fuzz_hex(hex in ".*") {
            let _ = parse_hex_bytes(&hex);
        }
    }
}
