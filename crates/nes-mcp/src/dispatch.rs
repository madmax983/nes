use core::fmt;
use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};

use nes_core::{Button, Command, CoreQuery, CoreSnapshot, NesCore, QueryResult};

use crate::output::{audio_chunk, frame_chunk, latest_output_metadata};

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
    Frame {
        seq: u64,
        bytes: usize,
    },
    Audio {
        seq: u64,
        samples: usize,
    },
    StateSlot {
        slot: String,
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
            execute_command(core, Command::SetControllerState(bits))?;
            Ok(DispatchOutput::ControllerState {
                controller_bits: core.controller_bits(),
            })
        }
        "press_button" => {
            let button = parse_button(params)?;
            execute_command(core, Command::PressButton(button))?;
            Ok(DispatchOutput::ControllerState {
                controller_bits: core.controller_bits(),
            })
        }
        "release_button" => {
            let button = parse_button(params)?;
            execute_command(core, Command::ReleaseButton(button))?;
            Ok(DispatchOutput::ControllerState {
                controller_bits: core.controller_bits(),
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
            let default_seq = latest_output_metadata().frame_seq.saturating_add(1);
            let requested_seq = parse_u64(params, "seq").unwrap_or(default_seq);
            let chunk = frame_chunk(requested_seq)
                .ok_or_else(|| DispatchError::Internal("frame chunk missing".to_owned()))?;
            Ok(DispatchOutput::Frame {
                seq: chunk.seq,
                bytes: chunk.rgba.len(),
            })
        }
        "get_audio_chunk" => {
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
                slots.get(&slot).copied()
            };
            if let Some(snapshot) = snapshot {
                core.load_state(&snapshot);
                Ok(DispatchOutput::StateSlot { slot })
            } else {
                Err(DispatchError::StateSlotNotFound(slot))
            }
        }
        "load_rom" | "disassemble_at" | "set_breakpoint" | "clear_breakpoint" => {
            Err(DispatchError::UnsupportedTool(tool_name.to_owned()))
        }
        _ => Err(DispatchError::UnknownTool(tool_name.to_owned())),
    }
}

fn execute_command(core: &mut NesCore, command: Command) -> Result<(), DispatchError> {
    core.execute(command)
        .map_err(|err| DispatchError::Core(err.to_string()))
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
