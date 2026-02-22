use core::fmt;

use crate::cpu::{Cpu, CpuError, CpuSnapshot};
use crate::replay::replay_commands;
use crate::rom::{RomError, parse_ines};
use crate::scheduler::{Scheduler, SchedulerSnapshot};

const DEFAULT_START_PC: u16 = 0xC000;
const DEFAULT_SPEED_PERMILLE: u16 = 1_000;
const BASE_FPS_MILLI: u32 = 60_000;
const PRG_16K_BYTES: usize = 16 * 1024;
const PRG_32K_BYTES: usize = 32 * 1024;

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
    Reset,
    PowerCycle,
    StepCpu,
    StepScanline,
    StepFrame,
    SetControllerState(u8),
    PressButton(Button),
    ReleaseButton(Button),
    SetSpeed(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreQuery {
    EmulatorState,
    Registers,
    Memory(u16),
    FpsMilli,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmulatorState {
    pub paused: bool,
    pub speed_permille: u16,
    pub controller_bits: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryResult {
    EmulatorState(EmulatorState),
    Registers(CpuSnapshot),
    Memory(u8),
    FpsMilli(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoreSnapshot {
    pub paused: bool,
    pub speed_permille: u16,
    pub controller_bits: u8,
    pub scheduler: SchedulerSnapshot,
    pub cpu: CpuSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RomLoadInfo {
    pub mapper_id: u8,
    pub prg_rom_bytes: usize,
    pub reset_pc: u16,
}

#[derive(Debug, Clone)]
pub struct NesCore {
    paused: bool,
    speed_permille: u16,
    scheduler: Scheduler,
    controller_bits: u8,
    cpu: Cpu,
    last_cpu_trace: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreError {
    UnsupportedCommand,
    InvalidSpeed(u16),
    RomLoadFailed(RomError),
    CpuStepFailed(CpuError),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCommand => f.write_str("unsupported command"),
            Self::InvalidSpeed(speed) => write!(f, "invalid speed multiplier: {speed}"),
            Self::RomLoadFailed(err) => write!(f, "rom load failed: {err}"),
            Self::CpuStepFailed(err) => write!(f, "cpu step failed: {err}"),
        }
    }
}

impl std::error::Error for CoreError {}

impl NesCore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            paused: false,
            speed_permille: DEFAULT_SPEED_PERMILLE,
            scheduler: Scheduler::new(),
            controller_bits: 0,
            cpu: Cpu::new(DEFAULT_START_PC),
            last_cpu_trace: None,
        }
    }

    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    #[must_use]
    pub const fn speed_permille(&self) -> u16 {
        self.speed_permille
    }

    #[must_use]
    pub fn total_cycles(&self) -> u64 {
        self.scheduler.total_cycles()
    }

    #[must_use]
    pub fn controller_bits(&self) -> u8 {
        self.controller_bits
    }

    pub fn load_cpu_bytes(&mut self, start: u16, bytes: &[u8]) {
        self.cpu.load_bytes(start, bytes);
    }

    #[must_use]
    pub fn cpu_pc(&self) -> u16 {
        self.cpu.pc()
    }

    #[must_use]
    pub fn cpu_a(&self) -> u8 {
        self.cpu.a()
    }

    #[must_use]
    pub fn cpu_x(&self) -> u8 {
        self.cpu.x()
    }

    #[must_use]
    pub fn last_cpu_trace(&self) -> Option<&str> {
        self.last_cpu_trace.as_deref()
    }

    #[must_use]
    pub fn cpu_snapshot(&self) -> CpuSnapshot {
        self.cpu.snapshot()
    }

    #[must_use]
    pub fn read_memory(&self, addr: u16) -> u8 {
        self.cpu.read_byte(addr)
    }

    #[must_use]
    pub fn fps_milli(&self) -> u32 {
        BASE_FPS_MILLI.saturating_mul(self.speed_permille as u32) / DEFAULT_SPEED_PERMILLE as u32
    }

    #[must_use]
    pub fn state_hash(&self) -> u64 {
        let paused = if self.paused { 1_u64 } else { 0_u64 };
        let cpu = self.cpu.snapshot();
        paused
            ^ self.scheduler.cpu_cycles().rotate_left(13)
            ^ self.scheduler.ppu_cycles().rotate_left(29)
            ^ self.scheduler.apu_cycles().rotate_left(47)
            ^ (self.speed_permille as u64).rotate_left(3)
            ^ (self.controller_bits as u64).rotate_left(7)
            ^ (cpu.pc as u64).rotate_left(19)
            ^ (cpu.a as u64).rotate_left(23)
            ^ (cpu.x as u64).rotate_left(31)
            ^ (cpu.y as u64).rotate_left(37)
            ^ (cpu.status as u64).rotate_left(41)
    }

    #[must_use]
    pub fn save_state(&self) -> CoreSnapshot {
        CoreSnapshot {
            paused: self.paused,
            speed_permille: self.speed_permille,
            controller_bits: self.controller_bits,
            scheduler: self.scheduler.snapshot(),
            cpu: self.cpu.snapshot(),
        }
    }

    pub fn load_state(&mut self, snapshot: &CoreSnapshot) {
        self.paused = snapshot.paused;
        self.speed_permille = snapshot.speed_permille;
        self.controller_bits = snapshot.controller_bits;
        self.scheduler.restore(snapshot.scheduler);
        self.cpu.restore(snapshot.cpu);
        self.last_cpu_trace = None;
    }

    pub fn replay(&mut self, commands: &[Command]) -> Result<(), CoreError> {
        replay_commands(self, commands)
    }

    pub fn load_ines_rom(&mut self, rom_bytes: &[u8]) -> Result<RomLoadInfo, CoreError> {
        let rom = parse_ines(rom_bytes).map_err(CoreError::RomLoadFailed)?;
        if rom.mapper_id != 0 {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedMapper(
                rom.mapper_id,
            )));
        }

        match rom.prg_rom.len() {
            PRG_16K_BYTES => {
                self.cpu.load_bytes(0x8000, rom.prg_rom);
                self.cpu.load_bytes(0xC000, rom.prg_rom);
            }
            PRG_32K_BYTES => {
                self.cpu.load_bytes(0x8000, rom.prg_rom);
            }
            other => {
                return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                    other,
                )));
            }
        }

        let reset_pc = {
            let lo = self.cpu.read_byte(0xFFFC);
            let hi = self.cpu.read_byte(0xFFFD);
            let pc = u16::from_le_bytes([lo, hi]);
            if pc == 0 { 0x8000 } else { pc }
        };

        self.paused = false;
        self.controller_bits = 0;
        self.scheduler.reset();
        self.cpu.reset(reset_pc);
        self.last_cpu_trace = None;

        Ok(RomLoadInfo {
            mapper_id: rom.mapper_id,
            prg_rom_bytes: rom.prg_rom.len(),
            reset_pc,
        })
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
            Command::Reset => {
                self.reset_runtime();
                Ok(())
            }
            Command::PowerCycle => {
                self.reset_runtime();
                self.speed_permille = DEFAULT_SPEED_PERMILLE;
                Ok(())
            }
            Command::StepCpu => {
                let trace = self
                    .cpu
                    .step_with_trace()
                    .map_err(CoreError::CpuStepFailed)?;
                self.last_cpu_trace = Some(trace);
                self.scheduler.step_cpu();
                Ok(())
            }
            Command::StepScanline => {
                self.scheduler.step_scanline();
                Ok(())
            }
            Command::StepFrame => {
                self.scheduler.step_frame();
                Ok(())
            }
            Command::SetControllerState(bits) => {
                self.controller_bits = bits;
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
            Command::SetSpeed(speed) => {
                if speed == 0 {
                    return Err(CoreError::InvalidSpeed(speed));
                }
                self.speed_permille = speed;
                Ok(())
            }
        }
    }

    #[must_use]
    pub fn query(&self, query: CoreQuery) -> QueryResult {
        match query {
            CoreQuery::EmulatorState => QueryResult::EmulatorState(EmulatorState {
                paused: self.paused,
                speed_permille: self.speed_permille,
                controller_bits: self.controller_bits,
            }),
            CoreQuery::Registers => QueryResult::Registers(self.cpu.snapshot()),
            CoreQuery::Memory(addr) => QueryResult::Memory(self.cpu.read_byte(addr)),
            CoreQuery::FpsMilli => QueryResult::FpsMilli(self.fps_milli()),
        }
    }

    fn reset_runtime(&mut self) {
        self.paused = false;
        self.controller_bits = 0;
        self.scheduler.reset();
        self.cpu.reset(DEFAULT_START_PC);
        self.last_cpu_trace = None;
    }
}

impl Default for NesCore {
    fn default() -> Self {
        Self::new()
    }
}
