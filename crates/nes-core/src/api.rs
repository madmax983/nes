use core::fmt;

use crate::scheduler::Scheduler;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Pause,
    Resume,
    StepCpu,
    StepFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreQuery {
    EmulatorState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmulatorState {
    pub paused: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryResult {
    EmulatorState(EmulatorState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NesCore {
    paused: bool,
    scheduler: Scheduler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreError {
    UnsupportedCommand,
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCommand => f.write_str("unsupported command"),
        }
    }
}

impl std::error::Error for CoreError {}

impl NesCore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            paused: false,
            scheduler: Scheduler::new(),
        }
    }

    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    #[must_use]
    pub fn total_cycles(&self) -> u64 {
        self.scheduler.total_cycles()
    }

    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let paused = if self.paused { 1_u64 } else { 0_u64 };
        paused
            ^ self.scheduler.cpu_cycles().rotate_left(13)
            ^ self.scheduler.ppu_cycles().rotate_left(29)
            ^ self.scheduler.apu_cycles().rotate_left(47)
    }

    pub fn execute(&mut self, command: Command) -> Result<(), CoreError> {
        match command {
            Command::Pause => {
                self.paused = true;
                Ok(())
            }
            Command::Resume => {
                self.paused = false;
                Ok(())
            }
            Command::StepCpu => {
                self.scheduler.step_cpu();
                Ok(())
            }
            Command::StepFrame => {
                self.scheduler.step_frame();
                Ok(())
            }
        }
    }

    #[must_use]
    pub fn query(&self, query: CoreQuery) -> QueryResult {
        match query {
            CoreQuery::EmulatorState => QueryResult::EmulatorState(EmulatorState {
                paused: self.paused,
            }),
        }
    }
}

impl Default for NesCore {
    fn default() -> Self {
        Self::new()
    }
}
