use core::fmt;

use crate::cpu::{Cpu, CpuError, CpuSnapshot, CpuWrite};
use crate::mapper::{Mmc1, Nrom, Uxrom};
use crate::ppu::{Ppu, PpuSnapshot};
use crate::replay::replay_commands;
use crate::rom::{RomError, parse_ines};
use crate::scheduler::{Scheduler, SchedulerSnapshot};

const DEFAULT_START_PC: u16 = 0xC000;
const DEFAULT_SPEED_PERMILLE: u16 = 1_000;
const BASE_FPS_MILLI: u32 = 60_000;
const PRG_16K_BYTES: usize = 16 * 1024;
const PRG_32K_BYTES: usize = 32 * 1024;
const PRG_BANK_BYTES: usize = 16 * 1024;
pub const FRAME_WIDTH: usize = 256;
pub const FRAME_HEIGHT: usize = 240;
pub const FRAME_RGBA_BYTES: usize = FRAME_WIDTH * FRAME_HEIGHT * 4;
pub const AUDIO_SAMPLE_RATE: u32 = 44_100;
pub const AUDIO_CHUNK_SAMPLES: usize = (AUDIO_SAMPLE_RATE as usize) / 60;
const AUDIO_MAX_AMPLITUDE: i16 = 12_000;
const AUDIO_MIN_FREQ_HZ: u32 = 55;
const AUDIO_MAX_FREQ_HZ: u32 = 1_760;

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
    PpuFrameCounter,
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
    PpuFrameCounter(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreSnapshot {
    pub paused: bool,
    pub speed_permille: u16,
    pub controller_bits: u8,
    pub scheduler: SchedulerSnapshot,
    pub ppu: PpuSnapshot,
    pub cpu: CpuSnapshot,
    audio_phase: u32,
    mapper: Option<LoadedMapper>,
    reset_pc: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RomLoadInfo {
    pub mapper_id: u8,
    pub prg_rom_bytes: usize,
    pub reset_pc: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LoadedMapper {
    Nrom(Nrom),
    Uxrom(Uxrom),
    Mmc1(Mmc1),
}

impl LoadedMapper {
    fn read_prg(&self, addr: u16) -> u8 {
        match self {
            Self::Nrom(mapper) => mapper.read_prg(addr),
            Self::Uxrom(mapper) => mapper.read_prg(addr),
            Self::Mmc1(mapper) => mapper.read_prg(addr),
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        match self {
            Self::Nrom(mapper) => mapper.write_prg(addr, value),
            Self::Uxrom(mapper) => mapper.write_prg(addr, value),
            Self::Mmc1(mapper) => mapper.write_prg(addr, value),
        }
    }
}

#[derive(Debug, Clone)]
pub struct NesCore {
    paused: bool,
    speed_permille: u16,
    scheduler: Scheduler,
    ppu: Ppu,
    controller_bits: u8,
    mapper: Option<LoadedMapper>,
    reset_pc: u16,
    cpu: Cpu,
    audio_phase: u32,
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
            ppu: Ppu::new(),
            controller_bits: 0,
            mapper: None,
            reset_pc: DEFAULT_START_PC,
            cpu: Cpu::new(DEFAULT_START_PC),
            audio_phase: 0,
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
        if start >= 0x8000 {
            self.mapper = None;
            self.reset_pc = DEFAULT_START_PC;
        }
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

    pub fn write_cpu_bus(&mut self, addr: u16, value: u8) {
        self.cpu.write_byte(addr, value);
        self.apply_cpu_writes(&[CpuWrite { addr, value }]);
        self.sync_ppu_register_image();
    }

    #[must_use]
    pub fn ppu_frame_counter(&self) -> u64 {
        self.ppu.frame_counter()
    }

    #[must_use]
    pub fn fps_milli(&self) -> u32 {
        BASE_FPS_MILLI.saturating_mul(self.speed_permille as u32) / DEFAULT_SPEED_PERMILLE as u32
    }

    #[must_use]
    pub fn framebuffer_rgba(&self) -> Vec<u8> {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        self.fill_framebuffer_rgba(&mut frame);
        frame
    }

    pub fn fill_framebuffer_rgba(&self, frame: &mut [u8]) {
        if frame.len() != FRAME_RGBA_BYTES {
            return;
        }

        let regs = self.cpu.snapshot();
        let frame_phase = self.ppu.frame_counter() as u8;
        let tint = self.ppu.ctrl() ^ self.ppu.mask() ^ self.controller_bits;
        let in_vblank = self.ppu.status() & 0x80 != 0;

        for y in 0..FRAME_HEIGHT {
            let y8 = y as u8;
            for x in 0..FRAME_WIDTH {
                let x8 = x as u8;
                let idx = (y * FRAME_WIDTH + x) * 4;

                let wave = x8.rotate_left(1).wrapping_add(y8.rotate_left(2));
                let motion = frame_phase.wrapping_mul(3).wrapping_add(wave);

                let mut r = x8.wrapping_add(regs.a).wrapping_add(motion);
                let mut g = y8.wrapping_add(regs.x).wrapping_add(tint);
                let mut b = (x8 ^ y8).wrapping_add(regs.y).wrapping_add(frame_phase);

                if in_vblank {
                    r = r.saturating_add(24);
                    g = g.saturating_add(24);
                    b = b.saturating_add(16);
                }

                frame[idx] = r;
                frame[idx + 1] = g;
                frame[idx + 2] = b;
                frame[idx + 3] = 0xFF;
            }
        }
    }

    #[must_use]
    pub fn audio_chunk_i16(&mut self) -> Vec<i16> {
        let mut samples = vec![0_i16; AUDIO_CHUNK_SAMPLES];
        self.fill_audio_chunk_i16(&mut samples);
        samples
    }

    pub fn fill_audio_chunk_i16(&mut self, samples: &mut [i16]) {
        let regs = self.cpu.snapshot();
        let button_energy = self.controller_bits.count_ones().saturating_mul(19);
        let mut freq_hz = 110_u32
            .saturating_add((regs.a as u32).saturating_mul(2))
            .saturating_add(regs.x as u32)
            .saturating_add((regs.y as u32) / 2)
            .saturating_add(button_energy);
        if self.ppu.status() & 0x80 != 0 {
            freq_hz = freq_hz.saturating_add(55);
        }
        freq_hz = freq_hz.clamp(AUDIO_MIN_FREQ_HZ, AUDIO_MAX_FREQ_HZ);

        let phase_step = ((u64::from(freq_hz) << 32) / u64::from(AUDIO_SAMPLE_RATE)) as u32;
        let duty_threshold = if self.controller_bits & Button::B.bit_mask() != 0 {
            0xC000_0000
        } else {
            0x8000_0000
        };

        let mut amplitude = if self.paused { 0 } else { AUDIO_MAX_AMPLITUDE };
        if self.ppu.status() & 0x80 != 0 {
            amplitude /= 2;
        }

        for sample in samples.iter_mut() {
            self.audio_phase = self.audio_phase.wrapping_add(phase_step);
            let high = self.audio_phase < duty_threshold;
            *sample = if high { amplitude } else { -amplitude };
        }
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
            ^ self.ppu.frame_counter().rotate_left(11)
            ^ (self.ppu.status() as u64).rotate_left(17)
            ^ (self.audio_phase as u64).rotate_left(59)
            ^ self.mapper_hash_component().rotate_left(53)
    }

    #[must_use]
    pub fn save_state(&self) -> CoreSnapshot {
        CoreSnapshot {
            paused: self.paused,
            speed_permille: self.speed_permille,
            controller_bits: self.controller_bits,
            scheduler: self.scheduler.snapshot(),
            ppu: self.ppu.snapshot(),
            cpu: self.cpu.snapshot(),
            audio_phase: self.audio_phase,
            mapper: self.mapper.clone(),
            reset_pc: self.reset_pc,
        }
    }

    pub fn load_state(&mut self, snapshot: &CoreSnapshot) {
        self.paused = snapshot.paused;
        self.speed_permille = snapshot.speed_permille;
        self.controller_bits = snapshot.controller_bits;
        self.scheduler.restore(snapshot.scheduler);
        self.ppu.restore(snapshot.ppu);
        self.cpu.restore(snapshot.cpu);
        self.audio_phase = snapshot.audio_phase;
        self.mapper = snapshot.mapper.clone();
        self.reset_pc = snapshot.reset_pc;
        self.sync_mapper_prg_window();
        self.sync_ppu_register_image();
        self.last_cpu_trace = None;
    }

    pub fn replay(&mut self, commands: &[Command]) -> Result<(), CoreError> {
        replay_commands(self, commands)
    }

    pub fn load_ines_rom(&mut self, rom_bytes: &[u8]) -> Result<RomLoadInfo, CoreError> {
        let rom = parse_ines(rom_bytes).map_err(CoreError::RomLoadFailed)?;
        let mapper = self.build_mapper(rom.mapper_id, rom.prg_rom)?;
        self.mapper = Some(mapper);
        self.sync_mapper_prg_window();

        let reset_pc = {
            let lo = self.cpu.read_byte(0xFFFC);
            let hi = self.cpu.read_byte(0xFFFD);
            let pc = u16::from_le_bytes([lo, hi]);
            if pc == 0 { 0x8000 } else { pc }
        };
        self.reset_pc = reset_pc;

        self.paused = false;
        self.controller_bits = 0;
        self.scheduler.reset();
        self.ppu.reset();
        self.cpu.reset(reset_pc);
        self.audio_phase = 0;
        self.sync_ppu_register_image();
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
                self.step_single_instruction()?;
                Ok(())
            }
            Command::StepScanline => {
                self.step_until_cpu_cycles(Scheduler::SCANLINE_CPU_CYCLES)?;
                Ok(())
            }
            Command::StepFrame => {
                self.step_until_cpu_cycles(Scheduler::FRAME_CPU_CYCLES)?;
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
            CoreQuery::PpuFrameCounter => QueryResult::PpuFrameCounter(self.ppu.frame_counter()),
        }
    }

    fn step_single_instruction(&mut self) -> Result<u64, CoreError> {
        let (trace, cpu_cycles) = self
            .cpu
            .step_with_trace_and_cycles()
            .map_err(CoreError::CpuStepFailed)?;
        self.last_cpu_trace = Some(trace);

        let writes = self.cpu.take_writes();
        self.apply_cpu_writes(&writes);

        let cpu_cycles = u64::from(cpu_cycles);
        self.scheduler.step_cpu_cycles(cpu_cycles);
        self.ppu.step_cycles(cpu_cycles.saturating_mul(3));
        if self.ppu.take_nmi_pending() {
            self.cpu.service_nmi();
        }
        self.sync_ppu_register_image();
        Ok(cpu_cycles)
    }

    fn step_until_cpu_cycles(&mut self, budget: u64) -> Result<(), CoreError> {
        let start = self.scheduler.cpu_cycles();
        while self.scheduler.cpu_cycles().saturating_sub(start) < budget {
            let _ = self.step_single_instruction()?;
        }
        Ok(())
    }

    fn reset_runtime(&mut self) {
        self.paused = false;
        self.controller_bits = 0;
        self.scheduler.reset();
        self.ppu.reset();
        self.cpu.reset(self.reset_pc);
        self.audio_phase = 0;
        self.sync_ppu_register_image();
        self.last_cpu_trace = None;
    }

    fn build_mapper(&self, mapper_id: u8, prg_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        match mapper_id {
            0 => self.build_nrom(prg_rom),
            1 => self.build_mmc1(prg_rom),
            2 => self.build_uxrom(prg_rom),
            _ => Err(CoreError::RomLoadFailed(RomError::UnsupportedMapper(
                mapper_id,
            ))),
        }
    }

    fn build_nrom(&self, prg_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        match prg_rom.len() {
            PRG_16K_BYTES | PRG_32K_BYTES => {
                Ok(LoadedMapper::Nrom(Nrom::from_prg_rom(prg_rom.to_vec())))
            }
            other => Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                other,
            ))),
        }
    }

    fn build_uxrom(&self, prg_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_BANK_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        Ok(LoadedMapper::Uxrom(Uxrom::from_prg_rom(prg_rom.to_vec())))
    }

    fn build_mmc1(&self, prg_rom: &[u8]) -> Result<LoadedMapper, CoreError> {
        if prg_rom.len() < PRG_32K_BYTES || !prg_rom.len().is_multiple_of(PRG_BANK_BYTES) {
            return Err(CoreError::RomLoadFailed(RomError::UnsupportedPrgLayout(
                prg_rom.len(),
            )));
        }
        Ok(LoadedMapper::Mmc1(Mmc1::from_prg_rom(prg_rom.to_vec(), 1)))
    }

    fn sync_mapper_prg_window(&mut self) {
        if let Some(mapper) = self.mapper.as_ref() {
            for addr in 0x8000..=0xFFFF {
                let value = mapper.read_prg(addr);
                self.cpu.write_byte(addr, value);
            }
        }
    }

    fn apply_cpu_writes(&mut self, writes: &[CpuWrite]) {
        let remap_needed = if let Some(mapper) = self.mapper.as_mut() {
            let mut wrote_prg = false;
            for write in writes {
                if write.addr >= 0x8000 {
                    mapper.write_prg(write.addr, write.value);
                    wrote_prg = true;
                }
            }
            wrote_prg
        } else {
            false
        };

        let ppu_changed = {
            let mut changed = false;
            for write in writes {
                if (0x2000..=0x3FFF).contains(&write.addr) {
                    self.ppu
                        .write_register(normalize_ppu_register_addr(write.addr), write.value);
                    changed = true;
                }
            }
            changed
        };

        if remap_needed {
            self.sync_mapper_prg_window();
        }
        if ppu_changed {
            self.sync_ppu_register_image();
        }
    }

    fn mapper_hash_component(&self) -> u64 {
        match self.mapper.as_ref() {
            None => 0,
            Some(LoadedMapper::Nrom(_)) => 0x10,
            Some(LoadedMapper::Uxrom(mapper)) => 0x20 ^ mapper.selected_bank() as u64,
            Some(LoadedMapper::Mmc1(mapper)) => 0x30 ^ mapper.selected_prg_bank() as u64,
        }
    }

    fn sync_ppu_register_image(&mut self) {
        self.cpu.write_byte(0x2000, self.ppu.ctrl());
        self.cpu.write_byte(0x2001, self.ppu.mask());
        self.cpu.write_byte(0x2002, self.ppu.status());
    }
}

#[must_use]
fn normalize_ppu_register_addr(addr: u16) -> u16 {
    0x2000 + ((addr - 0x2000) % 8)
}

impl Default for NesCore {
    fn default() -> Self {
        Self::new()
    }
}
