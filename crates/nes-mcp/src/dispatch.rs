use core::fmt;

use nes_core::{Command, NesCore};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchError {
    UnknownTool(String),
    Core(String),
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTool(name) => write!(f, "unknown tool: {name}"),
            Self::Core(msg) => write!(f, "core command failed: {msg}"),
        }
    }
}

impl std::error::Error for DispatchError {}

pub fn dispatch_tool(core: &mut NesCore, tool_name: &str) -> Result<(), DispatchError> {
    let command = match tool_name {
        "pause" => Command::Pause,
        "resume" => Command::Resume,
        "step_cpu" => Command::StepCpu,
        "step_frame" => Command::StepFrame,
        _ => return Err(DispatchError::UnknownTool(tool_name.to_owned())),
    };

    core.execute(command)
        .map_err(|err| DispatchError::Core(err.to_string()))
}
