use core::fmt;

use crate::replay::replay_commands;
use crate::scheduler::{Scheduler, SchedulerSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    A,
    B,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
}

impl Button {
    #[must_use]
    pub fn bit_mask(self) -> u8 {
        match self {
            Self::A => 0b0000_0001,
            Self::B => 0b0000_0010,
            Self::Select => 0b0000_0100,
            Self::Start => 0b0000_1000,
            Self::Up => 0b0001_0000,
            Self::Down => 0b0010_0000,
            Self::Left => 0b0100_0000,
            Self::Right => 0b1000_0000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Pause,
    Resume,
    StepCpu,
    StepFrame,
    PressButton(Button),
    ReleaseButton(Button),
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
pub struct CoreSnapshot {
    pub paused: bool,
    pub controller_bits: u8,
    pub scheduler: SchedulerSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NesCore {
    paused: bool,
    scheduler: Scheduler,
    controller_bits: u8,
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
            controller_bits: 0,
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
    pub fn controller_bits(&self) -> u8 {
        self.controller_bits
    }

    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let paused = if self.paused { 1_u64 } else { 0_u64 };
        paused
            ^ self.scheduler.cpu_cycles().rotate_left(13)
            ^ self.scheduler.ppu_cycles().rotate_left(29)
            ^ self.scheduler.apu_cycles().rotate_left(47)
            ^ (self.controller_bits as u64).rotate_left(7)
    }

    #[must_use]
    pub fn save_state(&self) -> CoreSnapshot {
        CoreSnapshot {
            paused: self.paused,
            controller_bits: self.controller_bits,
            scheduler: self.scheduler.snapshot(),
        }
    }

    pub fn load_state(&mut self, snapshot: &CoreSnapshot) {
        self.paused = snapshot.paused;
        self.controller_bits = snapshot.controller_bits;
        self.scheduler.restore(snapshot.scheduler);
    }

    pub fn replay(&mut self, commands: &[Command]) -> Result<(), CoreError> {
        replay_commands(self, commands)
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
            Command::PressButton(button) => {
                self.controller_bits |= button.bit_mask();
                Ok(())
            }
            Command::ReleaseButton(button) => {
                self.controller_bits &= !button.bit_mask();
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
