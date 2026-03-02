use core::fmt;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::sync::{Mutex, OnceLock};

use nes_core::{
    Button, Command, CoreQuery, CoreSnapshot, FRAME_HEIGHT, FRAME_WIDTH, NesCore, QueryResult,
};
use nes_dsl::{Mirroring, RomBuildOptions};

use crate::output::{
    audio_chunk, frame_chunk, latest_output_metadata, publish_audio, publish_frame,
};

pub type ToolParams = BTreeMap<String, String>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutput {
    Ack,
    CpuStep {
        trace: Option<String>,
        cpu_cycles: u64,
    },
    CycleCount {
        cpu_cycles: u64,
    },
    ControllerState {
        controller_bits: u8,
    },
    EmulatorState {
        paused: bool,
        speed_permille: u16,
        controller_bits: u8,
    },
    Registers {
        pc: u16,
        a: u8,
        x: u8,
        y: u8,
        sp: u8,
        status: u8,
    },
    Memory {
        address: u16,
        value: u8,
    },
    Fps {
        fps_milli: u32,
    },
    PpuFrameCounter {
        frame_counter: u64,
    },
    Frame {
        seq: u64,
        bytes: usize,
    },
    FrameCaptured {
        path: String,
        bytes: usize,
    },
    Audio {
        seq: u64,
        samples: usize,
    },
    StateSlot {
        slot: String,
    },
    RomLoaded {
        mapper_id: u8,
        prg_rom_bytes: usize,
        reset_pc: u16,
    },
    MacroExecuted {
        frames_elapsed: u64,
        final_controller_bits: u8,
    },
    DslAssembled {
        bytes_written: usize,
        label_count: usize,
        nmi_vector: u16,
        reset_vector: u16,
        irq_vector: u16,
    },
    DslRomLoaded {
        mapper_id: u8,
        prg_rom_bytes: usize,
        reset_pc: u16,
        rom_bytes: usize,
    },
    DslRomExported {
        path: String,
        bytes: usize,
        mapper_id: u8,
        prg_rom_bytes: usize,
    },
    DslRomExportedBase64 {
        rom_base64: String,
        bytes: usize,
        mapper_id: u8,
        prg_rom_bytes: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    UnknownTool(String),
    UnsupportedTool(String),
    InvalidParams(String),
    StateSlotNotFound(String),
    Core(String),
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
        "step_cpu" => {
            execute_command(core, Command::StepCpu)?;
            Ok(DispatchOutput::CpuStep {
                trace: core.last_cpu_trace().map(ToOwned::to_owned),
                cpu_cycles: core.total_cycles(),
            })
        }
        "step_scanline" => {
            execute_command(core, Command::StepScanline)?;
            Ok(DispatchOutput::CycleCount {
                cpu_cycles: core.total_cycles(),
            })
        }
        "step_frame" => {
            execute_command(core, Command::StepFrame)?;
            Ok(DispatchOutput::CycleCount {
                cpu_cycles: core.total_cycles(),
            })
        }
        "set_controller_state" => {
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
        "press_button" => {
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
        "release_button" => {
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
        "set_speed" => {
            let speed = parse_speed_permille(params)?;
            execute_command(core, Command::SetSpeed(speed))?;
            Ok(DispatchOutput::EmulatorState {
                paused: core.is_paused(),
                speed_permille: core.speed_permille(),
                controller_bits: core.controller_bits(),
            })
        }
        "get_fps" => match core.query(CoreQuery::FpsMilli) {
            QueryResult::FpsMilli(fps_milli) => Ok(DispatchOutput::Fps { fps_milli }),
            _ => Err(DispatchError::Internal(
                "unexpected core query result for get_fps".to_owned(),
            )),
        },
        "get_ppu_frame_counter" => match core.query(CoreQuery::PpuFrameCounter) {
            QueryResult::PpuFrameCounter(frame_counter) => {
                Ok(DispatchOutput::PpuFrameCounter { frame_counter })
            }
            _ => Err(DispatchError::Internal(
                "unexpected core query result for get_ppu_frame_counter".to_owned(),
            )),
        },
        "get_emulator_state" => match core.query(CoreQuery::EmulatorState) {
            QueryResult::EmulatorState(state) => Ok(DispatchOutput::EmulatorState {
                paused: state.paused,
                speed_permille: state.speed_permille,
                controller_bits: state.controller_bits,
            }),
            _ => Err(DispatchError::Internal(
                "unexpected core query result for get_emulator_state".to_owned(),
            )),
        },
        "read_registers" => match core.query(CoreQuery::Registers) {
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
        },
        "read_memory" => {
            let address = parse_u16(params, "address")?;
            match core.query(CoreQuery::Memory(address)) {
                QueryResult::Memory(value) => Ok(DispatchOutput::Memory { address, value }),
                _ => Err(DispatchError::Internal(
                    "unexpected core query result for read_memory".to_owned(),
                )),
            }
        }
        "get_frame" => {
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
        "capture_frame" => {
            let Some(path) = params.get("path").cloned() else {
                return Err(DispatchError::InvalidParams(
                    "path must be provided".to_owned(),
                ));
            };
            let rgba = core.framebuffer_rgba();
            write_frame_image(&path, FRAME_WIDTH, FRAME_HEIGHT, &rgba)?;
            Ok(DispatchOutput::FrameCaptured {
                path,
                bytes: rgba.len(),
            })
        }
        "get_audio_chunk" => {
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
        "save_state" => {
            let slot = parse_slot(params);
            let mut slots = saved_states()
                .lock()
                .map_err(|_| DispatchError::Internal("saved-state lock poisoned".to_owned()))?;
            slots.insert(slot.clone(), core.save_state());
            Ok(DispatchOutput::StateSlot { slot })
        }
        "load_state" => {
            let slot = parse_slot(params);
            let snapshot = {
                let slots = saved_states()
                    .lock()
                    .map_err(|_| DispatchError::Internal("saved-state lock poisoned".to_owned()))?;
                slots.get(&slot).cloned()
            };
            if let Some(snapshot) = snapshot {
                core.load_state(&snapshot);
                Ok(DispatchOutput::StateSlot { slot })
            } else {
                Err(DispatchError::StateSlotNotFound(slot))
            }
        }
        "load_rom" => {
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
        "run_macro" => {
            let Some(script) = params.get("script") else {
                return Err(DispatchError::InvalidParams(
                    "script must be provided".to_owned(),
                ));
            };
            let frames_elapsed = crate::macro_engine::execute_macro_script(core, script)
                .map_err(DispatchError::InvalidParams)?;
            Ok(DispatchOutput::MacroExecuted {
                frames_elapsed,
                final_controller_bits: core.controller_bits(),
            })
        }
        "assemble_6502_dsl" => {
            let source = parse_dsl_source(params)?;
            let assembled = nes_dsl::assemble(source).map_err(|err| {
                DispatchError::InvalidParams(format!("dsl assembly failed: {err}"))
            })?;
            Ok(DispatchOutput::DslAssembled {
                bytes_written: assembled.bytes.len(),
                label_count: assembled.labels.len(),
                nmi_vector: assembled.nmi_vector,
                reset_vector: assembled.reset_vector,
                irq_vector: assembled.irq_vector,
            })
        }
        "load_6502_dsl" => {
            let source = parse_dsl_source(params)?;
            let options = parse_dsl_rom_options(params)?;
            let rom = nes_dsl::build_ines_nrom_rom(source, &options).map_err(|err| {
                DispatchError::InvalidParams(format!("dsl rom build failed: {err}"))
            })?;
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
        "export_6502_dsl_rom" => {
            let source = parse_dsl_source(params)?;
            let options = parse_dsl_rom_options(params)?;
            let output_path = parse_required_string(params, "output_path")?;
            let rom = nes_dsl::build_ines_nrom_rom(source, &options).map_err(|err| {
                DispatchError::InvalidParams(format!("dsl rom build failed: {err}"))
            })?;
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
                path: output_path,
                bytes: rom.len(),
                mapper_id: 0,
                prg_rom_bytes,
            })
        }
        "export_6502_dsl_rom_base64" => {
            let source = parse_dsl_source(params)?;
            let options = parse_dsl_rom_options(params)?;
            let rom = nes_dsl::build_ines_nrom_rom(source, &options).map_err(|err| {
                DispatchError::InvalidParams(format!("dsl rom build failed: {err}"))
            })?;
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
        "disassemble_at" | "set_breakpoint" | "clear_breakpoint" => {
            Err(DispatchError::UnsupportedTool(tool_name.to_owned()))
        }
        _ => Err(DispatchError::UnknownTool(tool_name.to_owned())),
    }
}

fn execute_command(core: &mut NesCore, command: Command) -> Result<(), DispatchError> {
    core.execute(command)
        .map_err(|err| DispatchError::Core(err.to_string()))
}

fn sync_frame_output(core: &NesCore) {
    publish_frame(
        FRAME_WIDTH as u32,
        FRAME_HEIGHT as u32,
        core.framebuffer_rgba(),
    );
}

fn sync_audio_output(core: &mut NesCore) {
    publish_audio(core.audio_chunk_i16());
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
        encode_bmp(width, height, rgba)?
    } else {
        encode_ppm(width, height, rgba)
    };
    fs::write(path, image)
        .map_err(|err| DispatchError::InvalidParams(format!("unable to write '{path}': {err}")))
}

fn encode_ppm(width: usize, height: usize, rgba: &[u8]) -> Vec<u8> {
    let mut ppm = Vec::with_capacity(32 + width * height * 3);
    ppm.extend_from_slice(format!("P6\n{width} {height}\n255\n").as_bytes());
    for px in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&px[..3]);
    }
    ppm
}

fn encode_bmp(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, DispatchError> {
    let row_bytes = width
        .checked_mul(3)
        .ok_or_else(|| DispatchError::Internal("bmp row size overflow".to_owned()))?;
    let row_padding = (4 - (row_bytes % 4)) % 4;
    let stride = row_bytes
        .checked_add(row_padding)
        .ok_or_else(|| DispatchError::Internal("bmp stride overflow".to_owned()))?;
    let pixel_data_size = stride
        .checked_mul(height)
        .ok_or_else(|| DispatchError::Internal("bmp pixel data size overflow".to_owned()))?;
    let file_size = 54usize
        .checked_add(pixel_data_size)
        .ok_or_else(|| DispatchError::Internal("bmp file size overflow".to_owned()))?;

    let width_i32 = i32::try_from(width)
        .map_err(|_| DispatchError::Internal("bmp width out of range".to_owned()))?;
    let height_i32 = i32::try_from(height)
        .map_err(|_| DispatchError::Internal("bmp height out of range".to_owned()))?;
    let file_size_u32 = u32::try_from(file_size)
        .map_err(|_| DispatchError::Internal("bmp file size out of range".to_owned()))?;
    let pixel_data_size_u32 = u32::try_from(pixel_data_size)
        .map_err(|_| DispatchError::Internal("bmp pixel data size out of range".to_owned()))?;

    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_size_u32.to_le_bytes());
    bmp.extend_from_slice(&0u16.to_le_bytes());
    bmp.extend_from_slice(&0u16.to_le_bytes());
    bmp.extend_from_slice(&54u32.to_le_bytes());

    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&width_i32.to_le_bytes());
    bmp.extend_from_slice(&height_i32.to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&pixel_data_size_u32.to_le_bytes());
    bmp.extend_from_slice(&2_835u32.to_le_bytes());
    bmp.extend_from_slice(&2_835u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());

    for y in (0..height).rev() {
        let row_start = y * width * 4;
        for x in 0..width {
            let idx = row_start + x * 4;
            bmp.push(rgba[idx + 2]);
            bmp.push(rgba[idx + 1]);
            bmp.push(rgba[idx]);
        }
        bmp.extend(std::iter::repeat_n(0, row_padding));
    }

    Ok(bmp)
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

fn parse_required_string(params: &ToolParams, key: &str) -> Result<String, DispatchError> {
    let Some(value) = params.get(key).cloned() else {
        return Err(DispatchError::InvalidParams(format!(
            "{key} must be provided"
        )));
    };
    if value.trim().is_empty() {
        return Err(DispatchError::InvalidParams(format!(
            "{key} must not be empty"
        )));
    }
    Ok(value)
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
        out.push(ALPHABET[usize::from(((b0 & 0x03) << 4) | (b1 >> 4))] as char);
        out.push(ALPHABET[usize::from(((b1 & 0x0F) << 2) | (b2 >> 6))] as char);
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
        out.push(ALPHABET[usize::from(((b0 & 0x03) << 4) | (b1 >> 4))] as char);
        out.push(ALPHABET[usize::from((b1 & 0x0F) << 2)] as char);
        out.push('=');
    }

    out
}

fn parse_hex_bytes(raw: &str) -> Result<Vec<u8>, DispatchError> {
    let mut cleaned = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if !ch.is_ascii_whitespace() && ch != '_' {
            cleaned.push(ch);
        }
    }

    if !cleaned.len().is_multiple_of(2) {
        return Err(DispatchError::InvalidParams(
            "rom_hex must have an even number of hex digits".to_owned(),
        ));
    }

    let mut bytes = Vec::with_capacity(cleaned.len() / 2);
    let as_bytes = cleaned.as_bytes();
    let mut index = 0;
    while index < as_bytes.len() {
        let hi = decode_hex_nibble(as_bytes[index], index)?;
        let lo = decode_hex_nibble(as_bytes[index + 1], index + 1)?;
        bytes.push((hi << 4) | lo);
        index += 2;
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
        Button, DispatchError, Mirroring, ToolParams, encode_bmp, parse_button,
        parse_dsl_rom_options, parse_player2, parse_slot, parse_speed_permille, parse_u64,
        sync_audio_output, sync_frame_output,
    };
    use crate::output::latest_output_metadata;

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
        assert_eq!(parse_u64(&params(&[("frame", "not-a-number")]), "frame"), None);
    }

    #[test]
    fn parse_speed_permille_validates_and_converts_multiplier() {
        assert_eq!(
            parse_speed_permille(&params(&[("multiplier", "1.5")])).expect("valid multiplier"),
            1500
        );

        let bad_number =
            parse_speed_permille(&params(&[("multiplier", "abc")])).expect_err("invalid number");
        assert!(matches!(bad_number, DispatchError::InvalidParams(_)));

        let not_positive =
            parse_speed_permille(&params(&[("multiplier", "0")])).expect_err("non-positive");
        assert!(matches!(not_positive, DispatchError::InvalidParams(_)));

        let not_finite =
            parse_speed_permille(&params(&[("multiplier", "NaN")])).expect_err("not finite");
        assert!(matches!(not_finite, DispatchError::InvalidParams(_)));

        let out_of_range =
            parse_speed_permille(&params(&[("multiplier", "1000")])).expect_err("out of range");
        assert!(matches!(out_of_range, DispatchError::InvalidParams(_)));
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

        let vertical = parse_dsl_rom_options(&params(&[
            ("mirroring", "vertical"),
            ("chr_hex", "AA 55"),
        ]))
        .expect("vertical options");
        assert_eq!(vertical.mirroring, Mirroring::Vertical);
        assert_eq!(vertical.chr_rom, vec![0xAA, 0x55]);

        let err = parse_dsl_rom_options(&params(&[("mirroring", "diagonal")]))
            .expect_err("invalid mirroring");
        assert!(matches!(err, DispatchError::InvalidParams(_)));
    }

    #[test]
    fn encode_bmp_produces_expected_headers_and_pixel_order() {
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, // top row: red, green
            0, 0, 255, 255, 255, 255, 255, 255, // bottom row: blue, white
        ];
        let bmp = encode_bmp(2, 2, &rgba).expect("encode bmp");
        assert_eq!(&bmp[0..2], b"BM");
        assert_eq!(bmp.len(), 70);
        assert_eq!(u32::from_le_bytes([bmp[2], bmp[3], bmp[4], bmp[5]]), 70);
        assert_eq!(u32::from_le_bytes([bmp[10], bmp[11], bmp[12], bmp[13]]), 54);
        assert_eq!(u32::from_le_bytes([bmp[34], bmp[35], bmp[36], bmp[37]]), 16);
        assert_eq!(
            &bmp[54..],
            &[
                255, 0, 0, 255, 255, 255, 0, 0, // bottom row (BGR + padding)
                0, 0, 255, 0, 255, 0, 0, 0, // top row (BGR + padding)
            ]
        );
    }

    #[test]
    fn sync_outputs_publish_frame_and_audio_sequences() {
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
}
