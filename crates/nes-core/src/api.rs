use core::fmt;

use crate::apu::{Apu, ApuSnapshot, DmcDmaRequest};
use crate::cpu::{Cpu, CpuBusAccess, CpuBusAccessKind, CpuError, CpuSnapshot, CpuWrite};
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
const CONTROLLER_OPEN_BUS_MASK: u8 = 0x40;
pub const FRAME_WIDTH: usize = 256;
pub const FRAME_HEIGHT: usize = 240;
pub const FRAME_RGBA_BYTES: usize = FRAME_WIDTH * FRAME_HEIGHT * 4;
pub const AUDIO_SAMPLE_RATE: u32 = 44_100;
pub const AUDIO_CHUNK_SAMPLES: usize = (AUDIO_SAMPLE_RATE as usize) / 60;

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
    pub apu: ApuSnapshot,
    pub cpu: CpuSnapshot,
    controller_strobe: bool,
    controller_shift: u8,
    pending_oam_dma_page: Option<u8>,
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
    controller_strobe: bool,
    controller_shift: u8,
    mapper: Option<LoadedMapper>,
    reset_pc: u16,
    cpu: Cpu,
    apu: Apu,
    pending_oam_dma_page: Option<u8>,
    last_cpu_trace: Option<String>,
    last_cpu_bus_trace: Vec<CpuBusAccess>,
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
            controller_strobe: false,
            controller_shift: 0,
            mapper: None,
            reset_pc: DEFAULT_START_PC,
            cpu: Cpu::new(DEFAULT_START_PC),
            apu: Apu::new(),
            pending_oam_dma_page: None,
            last_cpu_trace: None,
            last_cpu_bus_trace: Vec::new(),
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
    pub fn last_cpu_bus_trace(&self) -> &[CpuBusAccess] {
        &self.last_cpu_bus_trace
    }

    #[must_use]
    pub fn read_memory(&self, addr: u16) -> u8 {
        match addr {
            0x2002 => self.ppu.status(),
            0x2004 => self.ppu.peek_oam_data_for_cpu_read(),
            0x2007 => self.ppu.peek_data_for_cpu_read(),
            0x4015 => self.apu.peek_status(),
            0x4016 => self.controller_port_sample(),
            _ => self.cpu.read_byte(addr),
        }
    }

    pub fn read_apu_status(&mut self) -> u8 {
        let status = self.apu.read_status();
        self.cpu.write_byte(0x4015, status);
        status
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
    pub fn ppu_scanline(&self) -> u16 {
        self.ppu.scanline()
    }

    #[must_use]
    pub fn ppu_dot(&self) -> u16 {
        self.ppu.dot()
    }

    #[must_use]
    pub fn ppu_total_cycles(&self) -> u64 {
        self.scheduler.ppu_cycles()
    }

    #[must_use]
    pub fn apu_total_cycles(&self) -> u64 {
        self.scheduler.apu_cycles()
    }

    #[must_use]
    pub fn apu_quarter_frame_ticks(&self) -> u64 {
        self.apu.quarter_frame_ticks()
    }

    #[must_use]
    pub fn apu_half_frame_ticks(&self) -> u64 {
        self.apu.half_frame_ticks()
    }

    #[must_use]
    pub fn apu_irq_pending(&self) -> bool {
        self.apu.irq_pending()
    }

    #[must_use]
    pub fn apu_dmc_irq_pending(&self) -> bool {
        self.apu.dmc_irq_pending()
    }

    #[must_use]
    pub fn apu_dmc_bytes_remaining(&self) -> u16 {
        self.apu.dmc_bytes_remaining()
    }

    #[must_use]
    pub fn apu_dmc_fetch_count(&self) -> u64 {
        self.apu.dmc_fetch_count()
    }

    #[must_use]
    pub fn apu_pulse_timer_reloads(&self) -> (u16, u16) {
        self.apu.pulse_timer_reloads()
    }

    #[must_use]
    pub fn ppu_oam_byte(&self, index: u8) -> u8 {
        self.ppu.oam_byte(index)
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
        self.ppu.render_rgba(frame);
    }

    #[must_use]
    pub fn audio_chunk_i16(&mut self) -> Vec<i16> {
        self.apu.drain_samples(AUDIO_CHUNK_SAMPLES, self.paused)
    }

    pub fn fill_audio_chunk_i16(&mut self, samples: &mut [i16]) {
        let drained = self.apu.drain_samples(samples.len(), self.paused);
        samples.copy_from_slice(&drained);
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
            ^ ((if self.controller_strobe { 1_u64 } else { 0_u64 }).rotate_left(15))
            ^ (self.controller_shift as u64).rotate_left(21)
            ^ (cpu.pc as u64).rotate_left(19)
            ^ (cpu.a as u64).rotate_left(23)
            ^ (cpu.x as u64).rotate_left(31)
            ^ (cpu.y as u64).rotate_left(37)
            ^ (cpu.status as u64).rotate_left(41)
            ^ self.ppu.frame_counter().rotate_left(11)
            ^ (self.ppu.status() as u64).rotate_left(17)
            ^ self.apu.total_cycles().rotate_left(59)
            ^ self.apu.quarter_frame_ticks().rotate_left(5)
            ^ self.apu.half_frame_ticks().rotate_left(9)
            ^ ((if self.apu.irq_pending() { 1_u64 } else { 0_u64 }).rotate_left(57))
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
            apu: self.apu.snapshot(),
            cpu: self.cpu.snapshot(),
            controller_strobe: self.controller_strobe,
            controller_shift: self.controller_shift,
            pending_oam_dma_page: self.pending_oam_dma_page,
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
        self.apu.restore(snapshot.apu.clone());
        self.cpu.restore(snapshot.cpu);
        self.controller_strobe = snapshot.controller_strobe;
        self.controller_shift = snapshot.controller_shift;
        self.pending_oam_dma_page = snapshot.pending_oam_dma_page;
        self.mapper = snapshot.mapper.clone();
        self.reset_pc = snapshot.reset_pc;
        self.sync_mapper_prg_window();
        self.sync_ppu_register_image();
        self.last_cpu_trace = None;
        self.last_cpu_bus_trace.clear();
    }

    pub fn replay(&mut self, commands: &[Command]) -> Result<(), CoreError> {
        replay_commands(self, commands)
    }

    pub fn load_ines_rom(&mut self, rom_bytes: &[u8]) -> Result<RomLoadInfo, CoreError> {
        let rom = parse_ines(rom_bytes).map_err(CoreError::RomLoadFailed)?;
        let mapper = self.build_mapper(rom.mapper_id, rom.prg_rom)?;
        self.mapper = Some(mapper);
        self.ppu.load_cartridge(rom.chr_rom, rom.mirroring);
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
        self.controller_strobe = false;
        self.controller_shift = 0;
        self.scheduler.reset();
        self.ppu.reset();
        self.apu.reset();
        self.cpu.reset(reset_pc);
        self.pending_oam_dma_page = None;
        self.sync_ppu_register_image();
        self.last_cpu_trace = None;
        self.last_cpu_bus_trace.clear();

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
                self.step_until_next_scanline()?;
                Ok(())
            }
            Command::StepFrame => {
                self.step_until_next_frame()?;
                Ok(())
            }
            Command::SetControllerState(bits) => {
                self.set_controller_bits(bits);
                self.sync_ppu_register_image();
                Ok(())
            }
            Command::PressButton(button) => {
                self.set_controller_bits(self.controller_bits | button.bit_mask());
                self.sync_ppu_register_image();
                Ok(())
            }
            Command::ReleaseButton(button) => {
                self.set_controller_bits(self.controller_bits & !button.bit_mask());
                self.sync_ppu_register_image();
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
            CoreQuery::Memory(addr) => QueryResult::Memory(self.read_memory(addr)),
            CoreQuery::FpsMilli => QueryResult::FpsMilli(self.fps_milli()),
            CoreQuery::PpuFrameCounter => QueryResult::PpuFrameCounter(self.ppu.frame_counter()),
        }
    }

    fn step_single_instruction(&mut self) -> Result<u64, CoreError> {
        self.sync_ppu_register_image();
        let (trace, cpu_cycles) = self
            .cpu
            .step_with_trace_and_cycles()
            .map_err(CoreError::CpuStepFailed)?;
        self.last_cpu_trace = Some(trace);
        self.last_cpu_bus_trace = self.cpu.take_bus_trace();

        let writes = self.cpu.take_writes();
        self.apply_cpu_writes(&writes);
        self.apply_cpu_reads();

        let cpu_cycles = u64::from(cpu_cycles);
        for _ in 0..cpu_cycles {
            self.step_hardware_cycle();
        }

        if let Some(page) = self.pending_oam_dma_page.take() {
            self.run_oam_dma(page);
        }
        if self.ppu.take_nmi_pending() {
            self.cpu.service_nmi();
            for _ in 0..7 {
                self.step_hardware_cycle();
            }
        } else if self.apu.irq_pending() && self.cpu.service_irq() {
            for _ in 0..7 {
                self.step_hardware_cycle();
            }
        }
        self.sync_ppu_register_image();
        Ok(cpu_cycles)
    }

    fn step_until_next_scanline(&mut self) -> Result<(), CoreError> {
        let start_scanline = self.ppu.scanline();
        while self.ppu.scanline() == start_scanline {
            let _ = self.step_single_instruction()?;
        }
        Ok(())
    }

    fn step_until_next_frame(&mut self) -> Result<(), CoreError> {
        let start_frame = self.ppu.frame_counter();
        while self.ppu.frame_counter() == start_frame {
            let _ = self.step_single_instruction()?;
        }
        Ok(())
    }

    fn reset_runtime(&mut self) {
        self.paused = false;
        self.controller_bits = 0;
        self.controller_strobe = false;
        self.controller_shift = 0;
        self.scheduler.reset();
        self.ppu.reset();
        self.apu.reset();
        self.cpu.reset(self.reset_pc);
        self.pending_oam_dma_page = None;
        self.sync_ppu_register_image();
        self.last_cpu_trace = None;
        self.last_cpu_bus_trace.clear();
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

        for write in writes {
            if (0x4000..=0x4017).contains(&write.addr) {
                if write.addr == 0x4014 {
                    self.pending_oam_dma_page = Some(write.value);
                    continue;
                }
                if write.addr == 0x4016 {
                    self.write_controller_strobe(write.value);
                    continue;
                }
                self.apu.write_register(write.addr, write.value);
            }
        }

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
        self.cpu
            .write_byte(0x2004, self.ppu.peek_oam_data_for_cpu_read());
        self.cpu
            .write_byte(0x2007, self.ppu.peek_data_for_cpu_read());
        self.cpu.write_byte(0x4015, self.apu.peek_status());
        self.cpu.write_byte(0x4016, self.controller_port_sample());
    }

    fn apply_cpu_reads(&mut self) {
        let mut saw_ppu_status_read = false;
        let mut ppu_data_reads = 0_u8;
        let mut apu_status_reads = 0_u8;
        let mut controller_reads = 0_u8;

        for access in &self.last_cpu_bus_trace {
            if access.kind != CpuBusAccessKind::Read {
                continue;
            }
            match access.addr {
                0x2002 => saw_ppu_status_read = true,
                0x2007 => ppu_data_reads = ppu_data_reads.saturating_add(1),
                0x4015 => apu_status_reads = apu_status_reads.saturating_add(1),
                0x4016 => controller_reads = controller_reads.saturating_add(1),
                _ => {}
            }
        }

        if saw_ppu_status_read {
            self.ppu.on_status_read();
        }
        for _ in 0..ppu_data_reads {
            let _ = self.ppu.consume_data_read();
        }
        for _ in 0..apu_status_reads {
            let _ = self.apu.read_status();
        }
        for _ in 0..controller_reads {
            self.consume_controller_read();
        }
    }

    fn set_controller_bits(&mut self, bits: u8) {
        self.controller_bits = bits;
        if self.controller_strobe {
            self.controller_shift = bits;
        }
    }

    fn write_controller_strobe(&mut self, value: u8) {
        let next_strobe = value & 1 != 0;
        if next_strobe {
            self.controller_strobe = true;
            self.controller_shift = self.controller_bits;
            return;
        }

        if self.controller_strobe {
            self.controller_shift = self.controller_bits;
        }
        self.controller_strobe = false;
    }

    fn controller_port_sample(&self) -> u8 {
        let bit = if self.controller_strobe {
            self.controller_bits & 1
        } else {
            self.controller_shift & 1
        };
        bit | CONTROLLER_OPEN_BUS_MASK
    }

    fn consume_controller_read(&mut self) {
        if !self.controller_strobe {
            self.controller_shift = (self.controller_shift >> 1) | 0x80;
        }
    }

    fn step_hardware_cycle(&mut self) {
        self.scheduler.step_cpu_cycle();
        self.scheduler.step_apu_cycle();
        let dmc_request = self.apu.step_cpu_cycle(self.paused);
        for _ in 0..3 {
            self.scheduler.step_ppu_cycle();
            self.ppu.step_dot();
        }
        if let Some(request) = dmc_request {
            self.apply_dmc_dma_request(request);
        }
    }

    fn apply_dmc_dma_request(&mut self, request: DmcDmaRequest) {
        let sample = self.cpu.read_byte(request.addr);
        self.apu.load_dmc_sample(sample);
        for _ in 0..request.stall_cycles {
            self.scheduler.step_cpu_cycle();
            self.scheduler.step_apu_cycle();
            let dmc_request = self.apu.step_cpu_cycle(self.paused);
            for _ in 0..3 {
                self.scheduler.step_ppu_cycle();
                self.ppu.step_dot();
            }
            if let Some(chained) = dmc_request {
                let byte = self.cpu.read_byte(chained.addr);
                self.apu.load_dmc_sample(byte);
            }
        }
    }

    fn run_oam_dma(&mut self, page: u8) {
        let mut bytes = [0_u8; 256];
        let base = u16::from(page) << 8;
        for (offset, slot) in bytes.iter_mut().enumerate() {
            *slot = self.cpu.read_byte(base.wrapping_add(offset as u16));
        }
        self.ppu.dma_oam(&bytes);

        let stall_cycles = if self.scheduler.cpu_cycles().is_multiple_of(2) {
            514
        } else {
            513
        };
        for _ in 0..stall_cycles {
            self.step_hardware_cycle();
        }
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
