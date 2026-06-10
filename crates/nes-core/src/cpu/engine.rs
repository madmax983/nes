//! 6502 CPU execution engine.
//!
//! The CPU model owns a 64K memory image, executes one instruction at a time,
//! records bus activity, and tracks write side-effects for MMIO application in
//! the outer core.

use core::fmt;
use std::cell::{Cell, RefCell};

use crate::cpu::status::Status;

const STACK_BASE: u16 = 0x0100;

// 6502 base CPU cycles per opcode. Dynamic penalties (branch taken/page cross)
// are applied separately where applicable.
const CPU_BASE_CYCLES: [u8; 256] = [
    // 0x00
    7, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 4, 4, 6, 6, // 0x10
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, // 0x20
    6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 4, 4, 6, 6, // 0x30
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, // 0x40
    6, 6, 2, 8, 3, 3, 5, 5, 3, 2, 2, 2, 3, 4, 6, 6, // 0x50
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, // 0x60
    6, 6, 2, 8, 3, 3, 5, 5, 4, 2, 2, 2, 5, 4, 6, 6, // 0x70
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, // 0x80
    2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4, // 0x90
    2, 6, 2, 6, 4, 4, 4, 4, 2, 5, 2, 5, 5, 5, 5, 5, // 0xA0
    2, 6, 2, 6, 3, 3, 3, 3, 2, 2, 2, 2, 4, 4, 4, 4, // 0xB0
    2, 5, 2, 5, 4, 4, 4, 4, 2, 4, 2, 4, 4, 4, 4, 4, // 0xC0
    2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6, // 0xD0
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7, // 0xE0
    2, 6, 2, 8, 3, 3, 5, 5, 2, 2, 2, 2, 4, 4, 6, 6, // 0xF0
    2, 5, 2, 8, 4, 4, 6, 6, 2, 4, 2, 7, 4, 4, 7, 7,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// CPU instruction execution errors.
pub enum CpuError {
    /// Opcode is not implemented.
    UnknownOpcode(u8),
}

impl fmt::Display for CpuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownOpcode(opcode) => write!(f, "unknown opcode 0x{opcode:02X}"),
        }
    }
}

impl std::error::Error for CpuError {}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Serializable register snapshot.
pub struct CpuSnapshot {
    /// Program counter.
    pub pc: u16,
    /// Accumulator register.
    pub a: u8,
    /// X index register.
    pub x: u8,
    /// Y index register.
    pub y: u8,
    /// Stack pointer.
    pub sp: u8,
    /// Raw status flags.
    pub status: u8,
    /// 2KB NES work RAM ($0000–$07FF).
    #[serde(
        serialize_with = "crate::serde_array::serialize_u8_array",
        deserialize_with = "crate::serde_array::deserialize_u8_array"
    )]
    pub work_ram: [u8; 2048],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// One observed CPU write on the CPU bus.
pub struct CpuWrite {
    /// Target address.
    pub addr: u16,
    /// Written value.
    pub value: u8,
    /// 1-based CPU bus cycle within the instruction when the write occurred.
    pub bus_cycle: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// CPU write that targeted PRG space (`>= 0x8000`).
pub struct CpuPrgWrite {
    /// Target address.
    pub addr: u16,
    /// Written value.
    pub value: u8,
    /// 1-based CPU bus cycle within the instruction when the write occurred.
    pub bus_cycle: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// MMIO read observed during instruction execution.
pub struct CpuMmioRead {
    /// MMIO address read by the CPU.
    pub addr: u16,
    /// 1-based CPU bus cycle within the instruction when the read occurred.
    pub bus_cycle: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Bus access classification used in instruction traces.
pub enum CpuBusAccessKind {
    /// Read transaction.
    Read,
    /// Write transaction.
    Write,
    /// Dummy read used for timing-accurate microphases.
    DummyRead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// One observed CPU bus transaction.
pub struct CpuBusAccess {
    /// Access address.
    pub addr: u16,
    /// Data value seen on the access.
    pub value: u8,
    /// Access kind.
    pub kind: CpuBusAccessKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TraceSnapshot {
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    p: u8,
    sp: u8,
}

#[derive(Debug, Clone)]
/// 6502 CPU state and execution engine.
pub struct Cpu {
    pc: u16,
    a: u8,
    x: u8,
    y: u8,
    sp: u8,
    status: Status,
    memory: [u8; 0x1_0000],
    writes: Vec<CpuWrite>,
    prg_writes: Vec<CpuPrgWrite>,
    /// Addresses read by the CPU during the last instruction, used by the outer
    /// core to apply MMIO read side-effects ($2002, $2007, $4015–$4017).
    /// Always populated regardless of `trace_enabled`. Uses interior mutability
    /// so `read()` can push without requiring `&mut self`.
    mmio_reads: RefCell<Vec<CpuMmioRead>>,
    bus_trace: RefCell<Vec<CpuBusAccess>>,
    bus_cycle: Cell<u8>,
    /// When `false`, skips bus-trace recording and trace-string formatting.
    /// Set to `false` for throughput-critical paths (AI training, rewind, netplay).
    trace_enabled: bool,
}

impl Cpu {
    /// Creates a CPU with canonical power-on register defaults.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::cpu::Cpu;
    /// let cpu = Cpu::new(0xC000);
    /// assert_eq!(cpu.pc(), 0xC000);
    /// ```
    #[must_use]
    pub fn new(start_pc: u16) -> Self {
        Self {
            pc: start_pc,
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,
            status: Status::with_bits(0x24),
            memory: [0; 0x1_0000],
            // ⚡ Bolt: Pre-allocate capacity to prevent repeated heap reallocations on the hot path
            writes: Vec::with_capacity(8),
            // ⚡ Bolt: Pre-allocate capacity to prevent repeated heap reallocations on the hot path
            prg_writes: Vec::with_capacity(8),
            // ⚡ Bolt: Pre-allocate capacity to prevent repeated heap reallocations on the hot path
            mmio_reads: RefCell::new(Vec::with_capacity(8)),
            // ⚡ Bolt: Pre-allocate capacity to prevent repeated heap reallocations on the hot path
            bus_trace: RefCell::new(Vec::with_capacity(8)),
            bus_cycle: Cell::new(0),
            trace_enabled: cfg!(debug_assertions),
        }
    }

    /// Enables or disables CPU trace generation.
    ///
    /// Defaults to `true` in debug builds and `false` in release builds.
    /// When disabled, bus-trace recording and trace-string formatting are skipped,
    /// eliminating `RefCell` borrow overhead and heap allocations in the hot path.
    pub fn set_trace_enabled(&mut self, enabled: bool) {
        self.trace_enabled = enabled;
    }

    /// Returns program counter.
    #[must_use]
    pub const fn pc(&self) -> u16 {
        self.pc
    }

    /// Returns accumulator.
    #[must_use]
    pub const fn a(&self) -> u8 {
        self.a
    }

    /// Returns X register.
    #[must_use]
    pub const fn x(&self) -> u8 {
        self.x
    }

    /// Returns Y register.
    #[must_use]
    pub const fn y(&self) -> u8 {
        self.y
    }

    /// Returns stack pointer.
    #[must_use]
    pub const fn sp(&self) -> u8 {
        self.sp
    }

    /// Reads a byte from CPU memory image.
    #[must_use]
    pub fn read_byte(&self, addr: u16) -> u8 {
        self.memory[normalize_cpu_addr(addr) as usize]
    }

    /// Writes a byte into CPU memory image.
    pub fn write_byte(&mut self, addr: u16, value: u8) {
        self.memory[normalize_cpu_addr(addr) as usize] = value;
    }

    /// Captures register snapshot including 2KB work RAM.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::cpu::Cpu;
    /// let cpu = Cpu::new(0xC000);
    /// let snap = cpu.snapshot();
    /// assert_eq!(snap.pc, 0xC000);
    /// ```
    #[must_use]
    pub fn snapshot(&self) -> CpuSnapshot {
        let mut work_ram = [0u8; 2048];
        work_ram.copy_from_slice(&self.memory[0..2048]);
        CpuSnapshot {
            pc: self.pc,
            a: self.a,
            x: self.x,
            y: self.y,
            sp: self.sp,
            status: self.status.bits(),
            work_ram,
        }
    }

    /// Restores CPU registers and 2KB work RAM, clearing transient trace buffers.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::cpu::Cpu;
    /// let mut cpu = Cpu::new(0xC000);
    /// let snap = cpu.snapshot();
    /// cpu.reset(0x8000);
    /// cpu.restore(snap);
    /// assert_eq!(cpu.pc(), 0xC000);
    /// ```
    pub fn restore(&mut self, snapshot: CpuSnapshot) {
        self.pc = snapshot.pc;
        self.a = snapshot.a;
        self.x = snapshot.x;
        self.y = snapshot.y;
        self.sp = snapshot.sp;
        self.status = Status::with_bits(snapshot.status);
        self.memory[0..2048].copy_from_slice(&snapshot.work_ram);
        self.writes.clear();
        self.prg_writes.clear();
        self.mmio_reads.borrow_mut().clear();
        self.bus_trace.borrow_mut().clear();
        self.bus_cycle.set(0);
    }

    /// Resets CPU registers and clears transient trace buffers.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::cpu::Cpu;
    /// let mut cpu = Cpu::new(0xC000);
    /// cpu.reset(0x8000);
    /// assert_eq!(cpu.pc(), 0x8000);
    /// ```
    pub fn reset(&mut self, start_pc: u16) {
        self.pc = start_pc;
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status = Status::with_bits(0x24);
        self.writes.clear();
        self.prg_writes.clear();
        self.mmio_reads.borrow_mut().clear();
        self.bus_trace.borrow_mut().clear();
        self.bus_cycle.set(0);
    }

    /// Copies raw bytes into CPU memory image at `start`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::cpu::Cpu;
    /// let mut cpu = Cpu::new(0xC000);
    /// cpu.load_bytes(0x2000, &[0xA9, 0xFF]);
    /// assert_eq!(cpu.read_byte(0x2000), 0xA9);
    /// ```
    pub fn load_bytes(&mut self, start: u16, bytes: &[u8]) {
        let start = start as usize;
        let end = start.saturating_add(bytes.len()).min(self.memory.len());
        let len = end.saturating_sub(start);
        self.memory[start..end].copy_from_slice(&bytes[..len]);
    }

    /// Services an NMI interrupt.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::cpu::Cpu;
    /// let mut cpu = Cpu::new(0xC000);
    /// cpu.load_bytes(0xFFFA, &[0x00, 0x80]); // NMI vector to 0x8000
    /// cpu.service_nmi();
    /// assert_eq!(cpu.pc(), 0x8000);
    /// ```
    pub fn service_nmi(&mut self) {
        let pc = self.pc;
        self.push((pc >> 8) as u8);
        self.push(pc as u8);
        self.push(self.status.bits_for_stack_push());
        self.status.set_interrupt_disable(true);
        self.pc = self.read_u16(0xFFFA);
    }

    /// Services an IRQ interrupt if not masked.
    ///
    /// Returns `true` when the IRQ was taken.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::cpu::Cpu;
    /// let mut cpu = Cpu::new(0xC000);
    /// cpu.load_bytes(0xFFFE, &[0x00, 0x80]); // IRQ vector to 0x8000
    /// // Interrupts are disabled by default on power-up (I flag set)
    /// assert!(!cpu.service_irq());
    /// ```
    pub fn service_irq(&mut self) -> bool {
        if self.status.interrupt_disable() {
            return false;
        }

        let pc = self.pc;
        self.push((pc >> 8) as u8);
        self.push(pc as u8);
        self.push(self.status.bits_for_stack_push());
        self.status.set_interrupt_disable(true);
        self.pc = self.read_u16(0xFFFE);
        true
    }

    /// Executes one instruction and returns formatted trace text.
    ///
    /// # Errors
    ///
    /// Returns [`CpuError`] when opcode decoding/execution fails.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::cpu::Cpu;
    /// let mut cpu = Cpu::new(0xC000);
    /// cpu.load_bytes(0xC000, &[0xEA]); // NOP
    /// let trace = cpu.step_with_trace().unwrap();
    /// assert!(trace.contains("NOP"));
    /// assert_eq!(cpu.pc(), 0xC001);
    /// ```
    pub fn step_with_trace(&mut self) -> Result<String, CpuError> {
        self.step_with_trace_and_cycles().map(|(trace, _)| trace)
    }

    /// Executes one instruction and returns trace text + cycle count.
    ///
    /// # Errors
    ///
    /// Returns [`CpuError`] when opcode decoding/execution fails.
    pub fn step_with_trace_and_cycles(&mut self) -> Result<(String, u8), CpuError> {
        self.writes.clear();
        self.prg_writes.clear();
        self.mmio_reads.borrow_mut().clear();
        self.bus_cycle.set(0);
        if self.trace_enabled {
            self.bus_trace.borrow_mut().clear();
        }

        let snapshot = TraceSnapshot {
            pc: self.pc,
            a: self.a,
            x: self.x,
            y: self.y,
            p: self.status.bits(),
            sp: self.sp,
        };

        let opcode = self.read(snapshot.pc);
        let cycles = self.instruction_cycles(snapshot, opcode);
        let step = match opcode {
            0x00 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("BRK"));
                let return_pc = snapshot.pc.wrapping_add(2);
                self.push((return_pc >> 8) as u8);
                self.push(return_pc as u8);
                self.push(self.status.bits_for_php());
                self.status.set_interrupt_disable(true);
                self.pc = self.read_u16(0xFFFE);
                Ok(trace)
            }
            0x01 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let ptr = zp.wrapping_add(self.x);
                let addr = self.read_u16_zp(ptr);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ORA (${zp:02X},X)"));
                self.ora_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x05 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ORA ${zp:02X}"));
                self.ora_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x06 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp as u16;
                let value = self.read(addr);
                let next = self.asl_value(value);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ASL ${zp:02X}"));
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x09 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("ORA #${imm:02X}"));
                self.ora_value(imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x0B => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("ANC #${imm:02X}"));
                self.and_value(imm);
                self.status.set_carry(self.status.negative());
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x0A => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("ASL A"));
                self.a = self.asl_value(self.a);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x0D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ORA ${addr:04X}"),
                );
                self.ora_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x0E => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let next = self.asl_value(value);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ASL ${addr:04X}"),
                );
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x11 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let base = self.read_u16_zp(zp);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ORA (${zp:02X}),Y"));
                self.ora_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x15 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ORA ${zp:02X},X"));
                self.ora_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x16 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let next = self.asl_value(value);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ASL ${zp:02X},X"));
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x19 => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ORA ${base:04X},Y"),
                );
                self.ora_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x1D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ORA ${base:04X},X"),
                );
                self.ora_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x1E => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr);
                let next = self.asl_value(value);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ASL ${base:04X},X"),
                );
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x03 | 0x07 | 0x0F | 0x13 | 0x17 | 0x1B | 0x1F => {
                let (addr, len, trace) =
                    self.decode_unofficial_rmw_addressing(snapshot, opcode, "SLO");
                let value = self.read(addr);
                let next = self.asl_value(value);
                self.write_and_track(addr, next);
                self.ora_value(next);
                self.pc = self.pc.wrapping_add(len);
                Ok(trace)
            }
            0x23 | 0x27 | 0x2F | 0x33 | 0x37 | 0x3B | 0x3F => {
                let (addr, len, trace) =
                    self.decode_unofficial_rmw_addressing(snapshot, opcode, "RLA");
                let value = self.read(addr);
                let next = self.rol_value(value);
                self.write_and_track(addr, next);
                self.and_value(next);
                self.pc = self.pc.wrapping_add(len);
                Ok(trace)
            }
            0x43 | 0x47 | 0x4F | 0x53 | 0x57 | 0x5B | 0x5F => {
                let (addr, len, trace) =
                    self.decode_unofficial_rmw_addressing(snapshot, opcode, "SRE");
                let value = self.read(addr);
                let next = self.lsr_value(value);
                self.write_and_track(addr, next);
                self.eor_value(next);
                self.pc = self.pc.wrapping_add(len);
                Ok(trace)
            }
            0x63 | 0x67 | 0x6F | 0x73 | 0x77 | 0x7B | 0x7F => {
                let (addr, len, trace) =
                    self.decode_unofficial_rmw_addressing(snapshot, opcode, "RRA");
                let value = self.read(addr);
                let next = self.ror_value(value);
                self.write_and_track(addr, next);
                self.adc_value(next);
                self.pc = self.pc.wrapping_add(len);
                Ok(trace)
            }
            0xC3 | 0xC7 | 0xCF | 0xD3 | 0xD7 | 0xDB | 0xDF => {
                let (addr, len, trace) =
                    self.decode_unofficial_rmw_addressing(snapshot, opcode, "DCP");
                let next = self.read(addr).wrapping_sub(1);
                self.write_and_track(addr, next);
                self.status.update_compare(self.a, next);
                self.pc = self.pc.wrapping_add(len);
                Ok(trace)
            }
            0xE3 | 0xE7 | 0xEF | 0xF3 | 0xF7 | 0xFB | 0xFF => {
                let (addr, len, trace) =
                    self.decode_unofficial_rmw_addressing(snapshot, opcode, "ISC");
                let next = self.read(addr).wrapping_add(1);
                self.write_and_track(addr, next);
                self.sbc_value(next);
                self.pc = self.pc.wrapping_add(len);
                Ok(trace)
            }
            0xA3 | 0xA7 | 0xAF | 0xB3 | 0xB7 | 0xBF => {
                let (addr, len, trace) = match opcode {
                    0xA3 => {
                        let zp = self.read(snapshot.pc.wrapping_add(1));
                        let ptr = zp.wrapping_add(self.x);
                        let addr = self.read_u16_zp(ptr);
                        let trace = self.maybe_trace(
                            snapshot,
                            &[opcode, zp],
                            format_args!("LAX (${zp:02X},X)"),
                        );
                        (addr, 2, trace)
                    }
                    0xA7 => {
                        let zp = self.read(snapshot.pc.wrapping_add(1));
                        let trace = self.maybe_trace(
                            snapshot,
                            &[opcode, zp],
                            format_args!("LAX ${zp:02X}"),
                        );
                        (zp as u16, 2, trace)
                    }
                    0xAF => {
                        let low = self.read(snapshot.pc.wrapping_add(1));
                        let high = self.read(snapshot.pc.wrapping_add(2));
                        let addr = u16::from_le_bytes([low, high]);
                        let trace = self.maybe_trace(
                            snapshot,
                            &[opcode, low, high],
                            format_args!("LAX ${addr:04X}"),
                        );
                        (addr, 3, trace)
                    }
                    0xB3 => {
                        let zp = self.read(snapshot.pc.wrapping_add(1));
                        let base = self.read_u16_zp(zp);
                        let addr = base.wrapping_add(self.y as u16);
                        let trace = self.maybe_trace(
                            snapshot,
                            &[opcode, zp],
                            format_args!("LAX (${zp:02X}),Y"),
                        );
                        (addr, 2, trace)
                    }
                    0xB7 => {
                        let zp = self.read(snapshot.pc.wrapping_add(1));
                        let addr = zp.wrapping_add(self.y) as u16;
                        let trace = self.maybe_trace(
                            snapshot,
                            &[opcode, zp],
                            format_args!("LAX ${zp:02X},Y"),
                        );
                        (addr, 2, trace)
                    }
                    _ => {
                        let low = self.read(snapshot.pc.wrapping_add(1));
                        let high = self.read(snapshot.pc.wrapping_add(2));
                        let base = u16::from_le_bytes([low, high]);
                        let addr = base.wrapping_add(self.y as u16);
                        let trace = self.maybe_trace(
                            snapshot,
                            &[opcode, low, high],
                            format_args!("LAX ${base:04X},Y"),
                        );
                        (addr, 3, trace)
                    }
                };
                let value = self.read(addr);
                self.a = value;
                self.x = value;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(len);
                Ok(trace)
            }
            0x83 | 0x87 | 0x8F | 0x97 => {
                let (addr, len, trace) = match opcode {
                    0x83 => {
                        let zp = self.read(snapshot.pc.wrapping_add(1));
                        let ptr = zp.wrapping_add(self.x);
                        let addr = self.read_u16_zp(ptr);
                        let trace = self.maybe_trace(
                            snapshot,
                            &[opcode, zp],
                            format_args!("SAX (${zp:02X},X)"),
                        );
                        (addr, 2, trace)
                    }
                    0x87 => {
                        let zp = self.read(snapshot.pc.wrapping_add(1));
                        let trace = self.maybe_trace(
                            snapshot,
                            &[opcode, zp],
                            format_args!("SAX ${zp:02X}"),
                        );
                        (zp as u16, 2, trace)
                    }
                    0x8F => {
                        let low = self.read(snapshot.pc.wrapping_add(1));
                        let high = self.read(snapshot.pc.wrapping_add(2));
                        let addr = u16::from_le_bytes([low, high]);
                        let trace = self.maybe_trace(
                            snapshot,
                            &[opcode, low, high],
                            format_args!("SAX ${addr:04X}"),
                        );
                        (addr, 3, trace)
                    }
                    _ => {
                        let zp = self.read(snapshot.pc.wrapping_add(1));
                        let addr = zp.wrapping_add(self.y) as u16;
                        let trace = self.maybe_trace(
                            snapshot,
                            &[opcode, zp],
                            format_args!("SAX ${zp:02X},Y"),
                        );
                        (addr, 2, trace)
                    }
                };
                self.write_and_track(addr, self.a & self.x);
                self.pc = self.pc.wrapping_add(len);
                Ok(trace)
            }
            0x21 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let ptr = zp.wrapping_add(self.x);
                let addr = self.read_u16_zp(ptr);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("AND (${zp:02X},X)"));
                self.and_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x25 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("AND ${zp:02X}"));
                self.and_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x29 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("AND #${imm:02X}"));
                self.and_value(imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x2D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("AND ${addr:04X}"),
                );
                self.and_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x2B => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("ANC #${imm:02X}"));
                self.and_value(imm);
                self.status.set_carry(self.status.negative());
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x31 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let base = self.read_u16_zp(zp);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("AND (${zp:02X}),Y"));
                self.and_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x35 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("AND ${zp:02X},X"));
                self.and_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x39 => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("AND ${base:04X},Y"),
                );
                self.and_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x3D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("AND ${base:04X},X"),
                );
                self.and_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x41 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let ptr = zp.wrapping_add(self.x);
                let addr = self.read_u16_zp(ptr);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("EOR (${zp:02X},X)"));
                self.eor_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x45 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("EOR ${zp:02X}"));
                self.eor_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x49 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("EOR #${imm:02X}"));
                self.eor_value(imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x4B => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("ALR #${imm:02X}"));
                self.and_value(imm);
                self.a = self.lsr_value(self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x4D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("EOR ${addr:04X}"),
                );
                self.eor_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x51 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let base = self.read_u16_zp(zp);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("EOR (${zp:02X}),Y"));
                self.eor_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x55 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("EOR ${zp:02X},X"));
                self.eor_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x59 => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("EOR ${base:04X},Y"),
                );
                self.eor_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x5D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("EOR ${base:04X},X"),
                );
                self.eor_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x20 => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let target = u16::from_le_bytes([low, high]);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("JSR ${target:04X}"),
                );

                let return_addr = snapshot.pc.wrapping_add(2);
                self.push((return_addr >> 8) as u8);
                self.push(return_addr as u8);
                self.pc = target;
                Ok(trace)
            }
            0x24 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let operand = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("BIT ${zp:02X}"));
                self.status.update_bit_test(self.a, operand);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x26 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp as u16;
                let value = self.read(addr);
                let next = self.rol_value(value);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ROL ${zp:02X}"));
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x2A => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("ROL A"));
                self.a = self.rol_value(self.a);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x2C => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let operand = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("BIT ${addr:04X}"),
                );
                self.status.update_bit_test(self.a, operand);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x2E => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let next = self.rol_value(value);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ROL ${addr:04X}"),
                );
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x28 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("PLP"));
                let bits = self.pull();
                self.status.restore_from_stack(bits);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x4C => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let target = u16::from_le_bytes([low, high]);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("JMP ${target:04X}"),
                );
                self.pc = target;
                Ok(trace)
            }
            0x46 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp as u16;
                let value = self.read(addr);
                let next = self.lsr_value(value);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("LSR ${zp:02X}"));
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x4A => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("LSR A"));
                self.a = self.lsr_value(self.a);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x4E => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let next = self.lsr_value(value);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("LSR ${addr:04X}"),
                );
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x40 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("RTI"));
                let bits = self.pull();
                self.status.restore_from_stack(bits);
                let low = self.pull();
                let high = self.pull();
                self.pc = u16::from_le_bytes([low, high]);
                Ok(trace)
            }
            0x48 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("PHA"));
                self.push(self.a);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x60 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("RTS"));

                let low = self.pull();
                let high = self.pull();
                let return_addr = u16::from_le_bytes([low, high]);
                self.pc = return_addr.wrapping_add(1);
                Ok(trace)
            }
            0x61 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let ptr = zp.wrapping_add(self.x);
                let addr = self.read_u16_zp(ptr);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ADC (${zp:02X},X)"));
                self.adc_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x6C => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let ptr = u16::from_le_bytes([low, high]);
                let lo = self.read(ptr);
                let hi_addr = (ptr & 0xFF00) | (ptr.wrapping_add(1) & 0x00FF);
                let hi = self.read(hi_addr);
                let target = u16::from_le_bytes([lo, hi]);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("JMP (${ptr:04X})"),
                );
                self.pc = target;
                Ok(trace)
            }
            0x66 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp as u16;
                let value = self.read(addr);
                let next = self.ror_value(value);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ROR ${zp:02X}"));
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x65 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ADC ${zp:02X}"));
                self.adc_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x69 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("ADC #${imm:02X}"));
                self.adc_value(imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x6A => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("ROR A"));
                self.a = self.ror_value(self.a);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x6B => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("ARR #${imm:02X}"));
                self.and_value(imm);
                self.a = self.ror_value(self.a);
                self.status.set_carry(self.a & 0x40 != 0);
                self.status
                    .set_overflow(((self.a >> 6) ^ (self.a >> 5)) & 1 != 0);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x6E => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let next = self.ror_value(value);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ROR ${addr:04X}"),
                );
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x6D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ADC ${addr:04X}"),
                );
                self.adc_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x68 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("PLA"));
                self.a = self.pull();
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x71 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let base = self.read_u16_zp(zp);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ADC (${zp:02X}),Y"));
                self.adc_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x75 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ADC ${zp:02X},X"));
                self.adc_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x78 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("SEI"));
                self.status.set_interrupt_disable(true);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x79 => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ADC ${base:04X},Y"),
                );
                self.adc_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x7D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ADC ${base:04X},X"),
                );
                self.adc_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x85 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("STA ${zp:02X}"));
                self.write_and_track(zp as u16, self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x81 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let ptr = zp.wrapping_add(self.x);
                let addr = self.read_u16_zp(ptr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("STA (${zp:02X},X)"));
                self.write_and_track(addr, self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x84 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("STY ${zp:02X}"));
                self.write_and_track(zp as u16, self.y);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x86 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("STX ${zp:02X}"));
                self.write_and_track(zp as u16, self.x);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x88 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("DEY"));
                self.y = self.y.wrapping_sub(1);
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x8A => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("TXA"));
                self.a = self.x;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x8D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("STA ${addr:04X}"),
                );
                self.write_and_track(addr, self.a);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x8C => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("STY ${addr:04X}"),
                );
                self.write_and_track(addr, self.y);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x8E => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("STX ${addr:04X}"),
                );
                self.write_and_track(addr, self.x);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x90 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, offset],
                    format_args!("BCC ${target:04X}"),
                );
                self.pc = if !self.status.carry() {
                    target
                } else {
                    next_pc
                };
                Ok(trace)
            }
            0x91 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let base = self.read_u16_zp(zp);
                let addr = base.wrapping_add(self.y as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("STA (${zp:02X}),Y"));
                self.write_and_track(addr, self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x95 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("STA ${zp:02X},X"));
                self.write_and_track(addr, self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x94 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("STY ${zp:02X},X"));
                self.write_and_track(addr, self.y);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x56 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let next = self.lsr_value(value);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("LSR ${zp:02X},X"));
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x96 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.y) as u16;
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("STX ${zp:02X},Y"));
                self.write_and_track(addr, self.x);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x36 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let next = self.rol_value(value);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ROL ${zp:02X},X"));
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x98 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("TYA"));
                self.a = self.y;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x99 => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.y as u16);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("STA ${base:04X},Y"),
                );
                self.write_and_track(addr, self.a);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x9A => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("TXS"));
                self.sp = self.x;
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x9D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("STA ${base:04X},X"),
                );
                self.write_and_track(addr, self.a);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x5E => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr);
                let next = self.lsr_value(value);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("LSR ${base:04X},X"),
                );
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x3E => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr);
                let next = self.rol_value(value);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ROL ${base:04X},X"),
                );
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x76 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let next = self.ror_value(value);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("ROR ${zp:02X},X"));
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x7E => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr);
                let next = self.ror_value(value);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("ROR ${base:04X},X"),
                );
                self.write_and_track(addr, next);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x08 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("PHP"));
                self.push(self.status.bits_for_php());
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xA0 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("LDY #${imm:02X}"));
                self.y = imm;
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xA1 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let ptr = zp.wrapping_add(self.x);
                let addr = self.read_u16_zp(ptr);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("LDA (${zp:02X},X)"));
                self.a = value;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xA2 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("LDX #${imm:02X}"));
                self.x = imm;
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xA4 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("LDY ${zp:02X}"));
                self.y = value;
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xA5 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("LDA ${zp:02X}"));
                self.a = value;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xA6 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("LDX ${zp:02X}"));
                self.x = value;
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xA8 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("TAY"));
                self.y = self.a;
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xA9 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("LDA #${imm:02X}"));
                self.a = imm;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xAA => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("TAX"));
                self.x = self.a;
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xAB => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("LAX #${imm:02X}"));
                self.a = imm;
                self.x = imm;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xAC => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("LDY ${addr:04X}"),
                );
                self.y = self.read(addr);
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xAD => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("LDA ${addr:04X}"),
                );
                self.a = self.read(addr);
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xAE => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("LDX ${addr:04X}"),
                );
                self.x = self.read(addr);
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xB0 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, offset],
                    format_args!("BCS ${target:04X}"),
                );
                self.pc = if self.status.carry() { target } else { next_pc };
                Ok(trace)
            }
            0xBA => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("TSX"));
                self.x = self.sp;
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xB1 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let base = self.read_u16_zp(zp);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("LDA (${zp:02X}),Y"));
                self.a = value;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xB4 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("LDY ${zp:02X},X"));
                self.y = value;
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xB5 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("LDA ${zp:02X},X"));
                self.a = value;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xB6 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.y) as u16;
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("LDX ${zp:02X},Y"));
                self.x = value;
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xB9 => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("LDA ${base:04X},Y"),
                );
                self.a = value;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xBD => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("LDA ${base:04X},X"),
                );
                self.a = self.read(addr);
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xBC => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("LDY ${base:04X},X"),
                );
                self.y = value;
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xBE => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("LDX ${base:04X},Y"),
                );
                self.x = value;
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xC0 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("CPY #${imm:02X}"));
                self.status.update_compare(self.y, imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xC1 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let ptr = zp.wrapping_add(self.x);
                let addr = self.read_u16_zp(ptr);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("CMP (${zp:02X},X)"));
                self.status.update_compare(self.a, value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xC4 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("CPY ${zp:02X}"));
                self.status.update_compare(self.y, value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xC5 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("CMP ${zp:02X}"));
                self.status.update_compare(self.a, value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xC6 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp as u16;
                let value = self.read(addr).wrapping_sub(1);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("DEC ${zp:02X}"));
                self.write_and_track(addr, value);
                self.status.update_zn(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xC8 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("INY"));
                self.y = self.y.wrapping_add(1);
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xC9 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("CMP #${imm:02X}"));
                self.status.update_compare(self.a, imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xCC => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("CPY ${addr:04X}"),
                );
                self.status.update_compare(self.y, value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xCD => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("CMP ${addr:04X}"),
                );
                self.status.update_compare(self.a, value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xCA => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("DEX"));
                self.x = self.x.wrapping_sub(1);
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xCB => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("AXS #${imm:02X}"));
                let anded = self.a & self.x;
                self.x = anded.wrapping_sub(imm);
                self.status.set_carry(anded >= imm);
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xD0 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, offset],
                    format_args!("BNE ${target:04X}"),
                );
                self.pc = if !self.status.zero() { target } else { next_pc };
                Ok(trace)
            }
            0xD1 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let base = self.read_u16_zp(zp);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("CMP (${zp:02X}),Y"));
                self.status.update_compare(self.a, value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xD5 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("CMP ${zp:02X},X"));
                self.status.update_compare(self.a, value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xD6 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr).wrapping_sub(1);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("DEC ${zp:02X},X"));
                self.write_and_track(addr, value);
                self.status.update_zn(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xD8 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("CLD"));
                self.status.set_decimal(false);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xD9 => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("CMP ${base:04X},Y"),
                );
                self.status.update_compare(self.a, value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xDD => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("CMP ${base:04X},X"),
                );
                self.status.update_compare(self.a, value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xDE => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr).wrapping_sub(1);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("DEC ${base:04X},X"),
                );
                self.write_and_track(addr, value);
                self.status.update_zn(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xE0 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("CPX #${imm:02X}"));
                self.status.update_compare(self.x, imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xE1 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let ptr = zp.wrapping_add(self.x);
                let addr = self.read_u16_zp(ptr);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("SBC (${zp:02X},X)"));
                self.sbc_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xE4 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("CPX ${zp:02X}"));
                self.status.update_compare(self.x, value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xE5 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("SBC ${zp:02X}"));
                self.sbc_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xE6 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp as u16;
                let value = self.read(addr).wrapping_add(1);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("INC ${zp:02X}"));
                self.write_and_track(addr, value);
                self.status.update_zn(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xE8 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("INX"));
                self.x = self.x.wrapping_add(1);
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xE9 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("SBC #${imm:02X}"));
                self.sbc_value(imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xEB => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace =
                    self.maybe_trace(snapshot, &[opcode, imm], format_args!("SBC #${imm:02X}"));
                self.sbc_value(imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xEA => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("NOP"));
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xEE => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr).wrapping_add(1);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("INC ${addr:04X}"),
                );
                self.write_and_track(addr, value);
                self.status.update_zn(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xEC => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("CPX ${addr:04X}"),
                );
                self.status.update_compare(self.x, value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xED => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("SBC ${addr:04X}"),
                );
                self.sbc_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xF0 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, offset],
                    format_args!("BEQ ${target:04X}"),
                );
                self.pc = if self.status.zero() { target } else { next_pc };
                Ok(trace)
            }
            0xF1 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let base = self.read_u16_zp(zp);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("SBC (${zp:02X}),Y"));
                self.sbc_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xF5 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("SBC ${zp:02X},X"));
                self.sbc_value(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xF6 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let value = self.read(addr).wrapping_add(1);
                let trace =
                    self.maybe_trace(snapshot, &[opcode, zp], format_args!("INC ${zp:02X},X"));
                self.write_and_track(addr, value);
                self.status.update_zn(value);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xF9 => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.y as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("SBC ${base:04X},Y"),
                );
                self.sbc_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xCE => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let value = self.read(addr).wrapping_sub(1);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("DEC ${addr:04X}"),
                );
                self.write_and_track(addr, value);
                self.status.update_zn(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xFE => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr).wrapping_add(1);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("INC ${base:04X},X"),
                );
                self.write_and_track(addr, value);
                self.status.update_zn(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xFD => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let value = self.read(addr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("SBC ${base:04X},X"),
                );
                self.sbc_value(value);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x10 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, offset],
                    format_args!("BPL ${target:04X}"),
                );
                self.pc = if !self.status.negative() {
                    target
                } else {
                    next_pc
                };
                Ok(trace)
            }
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("NOP"));
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x18 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("CLC"));
                self.status.set_carry(false);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x30 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, offset],
                    format_args!("BMI ${target:04X}"),
                );
                self.pc = if self.status.negative() {
                    target
                } else {
                    next_pc
                };
                Ok(trace)
            }
            0x38 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("SEC"));
                self.status.set_carry(true);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x58 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("CLI"));
                self.status.set_interrupt_disable(false);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x50 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, offset],
                    format_args!("BVC ${target:04X}"),
                );
                self.pc = if !self.status.overflow() {
                    target
                } else {
                    next_pc
                };
                Ok(trace)
            }
            0x80 | 0x82 | 0x89 | 0xC2 | 0xE2 => {
                let operand = self.read(snapshot.pc.wrapping_add(1));
                let trace = self.maybe_trace(snapshot, &[opcode, operand], format_args!("NOP"));
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x04 | 0x44 | 0x64 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let _ = self.read(zp as u16);
                let trace = self.maybe_trace(snapshot, &[opcode, zp], format_args!("NOP"));
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x14 | 0x34 | 0x54 | 0x74 | 0xD4 | 0xF4 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let _ = self.read(addr);
                let trace = self.maybe_trace(snapshot, &[opcode, zp], format_args!("NOP"));
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xB8 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("CLV"));
                self.status.set_overflow(false);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x70 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, offset],
                    format_args!("BVS ${target:04X}"),
                );
                self.pc = if self.status.overflow() {
                    target
                } else {
                    next_pc
                };
                Ok(trace)
            }
            0x0C => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let _ = self.read(addr);
                let trace = self.maybe_trace(snapshot, &[opcode, low, high], format_args!("NOP"));
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let _ = self.read(addr);
                let trace = self.maybe_trace(snapshot, &[opcode, low, high], format_args!("NOP"));
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xF8 => {
                let trace = self.maybe_trace(snapshot, &[opcode], format_args!("SED"));
                self.status.set_decimal(true);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            _ => Err(CpuError::UnknownOpcode(opcode)),
        };

        step.map(|trace| {
            if self.trace_enabled {
                self.pad_microphase_to_cycle_count(snapshot.pc, cycles);
            }
            (trace, cycles)
        })
    }

    fn read(&self, addr: u16) -> u8 {
        let resolved = normalize_cpu_addr(addr);
        let value = self.memory[resolved as usize];
        let bus_cycle = self.observe_bus_access(resolved, value, CpuBusAccessKind::Read);
        // Track MMIO reads unconditionally so apply_cpu_reads can fire side-effects
        // ($2002 VBlank clear, $2007 PPU data, $4015 APU status, $4016/$4017 controllers)
        // regardless of trace_enabled.
        match resolved {
            0x2002 | 0x2007 | 0x4015 | 0x4016 | 0x4017 => {
                self.mmio_reads.borrow_mut().push(CpuMmioRead {
                    addr: resolved,
                    bus_cycle,
                });
            }
            _ => {}
        }
        value
    }

    fn peek(&self, addr: u16) -> u8 {
        self.memory[normalize_cpu_addr(addr) as usize]
    }

    fn read_u16(&self, addr: u16) -> u16 {
        let low = self.read(addr);
        let high = self.read(addr.wrapping_add(1));
        u16::from_le_bytes([low, high])
    }

    fn read_u16_zp(&self, addr: u8) -> u16 {
        let low = self.read(addr as u16);
        let high = self.read(addr.wrapping_add(1) as u16);
        u16::from_le_bytes([low, high])
    }

    fn peek_u16_zp(&self, addr: u8) -> u16 {
        let low = self.peek(addr as u16);
        let high = self.peek(addr.wrapping_add(1) as u16);
        u16::from_le_bytes([low, high])
    }

    fn write_and_track(&mut self, addr: u16, value: u8) {
        self.write_byte(addr, value);
        let bus_cycle = self.observe_bus_access(addr, value, CpuBusAccessKind::Write);
        self.writes.push(CpuWrite {
            addr,
            value,
            bus_cycle,
        });
        if addr >= 0x8000 {
            self.prg_writes.push(CpuPrgWrite {
                addr,
                value,
                bus_cycle,
            });
        }
    }

    fn lsr_value(&mut self, value: u8) -> u8 {
        self.status.set_carry(value & 0x01 != 0);
        let next = value >> 1;
        self.status.update_zn(next);
        next
    }

    fn asl_value(&mut self, value: u8) -> u8 {
        self.status.set_carry(value & 0x80 != 0);
        let next = value << 1;
        self.status.update_zn(next);
        next
    }

    fn rol_value(&mut self, value: u8) -> u8 {
        let carry_in = u8::from(self.status.carry());
        self.status.set_carry(value & 0x80 != 0);
        let next = (value << 1) | carry_in;
        self.status.update_zn(next);
        next
    }

    fn ror_value(&mut self, value: u8) -> u8 {
        let carry_in = if self.status.carry() { 0x80 } else { 0x00 };
        self.status.set_carry(value & 0x01 != 0);
        let next = (value >> 1) | carry_in;
        self.status.update_zn(next);
        next
    }

    fn adc_value(&mut self, value: u8) {
        let a = self.a;
        let carry_in = u8::from(self.status.carry());
        let sum = u16::from(a) + u16::from(value) + u16::from(carry_in);
        let result = sum as u8;
        self.status.set_carry(sum > 0xFF);
        self.status
            .set_overflow(((!(a ^ value) & (a ^ result)) & 0x80) != 0);
        self.a = result;
        self.status.update_zn(self.a);
    }

    fn sbc_value(&mut self, value: u8) {
        self.adc_value(value ^ 0xFF);
    }

    fn ora_value(&mut self, value: u8) {
        self.a |= value;
        self.status.update_zn(self.a);
    }

    fn and_value(&mut self, value: u8) {
        self.a &= value;
        self.status.update_zn(self.a);
    }

    fn eor_value(&mut self, value: u8) {
        self.a ^= value;
        self.status.update_zn(self.a);
    }

    fn instruction_cycles(&self, snapshot: TraceSnapshot, opcode: u8) -> u8 {
        let mut cycles = CPU_BASE_CYCLES[opcode as usize];

        if Self::is_branch_opcode(opcode) {
            if self.branch_taken(snapshot, opcode) {
                cycles = cycles.saturating_add(1);
                let offset = self.peek(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                if page_crossed(next_pc, target) {
                    cycles = cycles.saturating_add(1);
                }
            }
            return cycles;
        }

        if Self::abs_x_page_penalty_opcode(opcode) {
            let base = self.absolute_operand_base(snapshot);
            let indexed = base.wrapping_add(snapshot.x as u16);
            if page_crossed(base, indexed) {
                cycles = cycles.saturating_add(1);
            }
            return cycles;
        }

        if Self::abs_y_page_penalty_opcode(opcode) {
            let base = self.absolute_operand_base(snapshot);
            let indexed = base.wrapping_add(snapshot.y as u16);
            if page_crossed(base, indexed) {
                cycles = cycles.saturating_add(1);
            }
            return cycles;
        }

        if Self::indirect_y_page_penalty_opcode(opcode) {
            let zp = self.peek(snapshot.pc.wrapping_add(1));
            let base = self.peek_u16_zp(zp);
            let indexed = base.wrapping_add(snapshot.y as u16);
            if page_crossed(base, indexed) {
                cycles = cycles.saturating_add(1);
            }
        }

        cycles
    }

    /// **Optimization:** Returns the absolute address, instruction length, and pre-formatted trace
    /// without allocating intermediate `Vec` or `String` on the hot CPU path, by delegating
    /// to `maybe_trace` which uses `format_args!` internally.
    fn decode_unofficial_rmw_addressing(
        &self,
        snapshot: TraceSnapshot,
        opcode: u8,
        mnemonic: &str,
    ) -> (u16, u16, String) {
        match opcode & 0x1F {
            0x03 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let ptr = zp.wrapping_add(snapshot.x);
                let addr = self.read_u16_zp(ptr);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, zp],
                    format_args!("{mnemonic} (${zp:02X},X)"),
                );
                (addr, 2, trace)
            }
            0x07 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, zp],
                    format_args!("{mnemonic} ${zp:02X}"),
                );
                (zp as u16, 2, trace)
            }
            0x0F => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("{mnemonic} ${addr:04X}"),
                );
                (addr, 3, trace)
            }
            0x13 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let base = self.read_u16_zp(zp);
                let addr = base.wrapping_add(snapshot.y as u16);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, zp],
                    format_args!("{mnemonic} (${zp:02X}),Y"),
                );
                (addr, 2, trace)
            }
            0x17 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(snapshot.x) as u16;
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, zp],
                    format_args!("{mnemonic} ${zp:02X},X"),
                );
                (addr, 2, trace)
            }
            0x1B => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(snapshot.y as u16);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("{mnemonic} ${base:04X},Y"),
                );
                (addr, 3, trace)
            }
            0x1F => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(snapshot.x as u16);
                let trace = self.maybe_trace(
                    snapshot,
                    &[opcode, low, high],
                    format_args!("{mnemonic} ${base:04X},X"),
                );
                (addr, 3, trace)
            }
            _ => unreachable!(),
        }
    }

    fn absolute_operand_base(&self, snapshot: TraceSnapshot) -> u16 {
        let low = self.peek(snapshot.pc.wrapping_add(1));
        let high = self.peek(snapshot.pc.wrapping_add(2));
        u16::from_le_bytes([low, high])
    }

    fn branch_taken(&self, snapshot: TraceSnapshot, opcode: u8) -> bool {
        const CARRY_BIT: u8 = 0x01;
        const ZERO_BIT: u8 = 0x02;
        const OVERFLOW_BIT: u8 = 0x40;
        const NEGATIVE_BIT: u8 = 0x80;

        match opcode {
            0x10 => snapshot.p & NEGATIVE_BIT == 0, // BPL
            0x30 => snapshot.p & NEGATIVE_BIT != 0, // BMI
            0x50 => snapshot.p & OVERFLOW_BIT == 0, // BVC
            0x70 => snapshot.p & OVERFLOW_BIT != 0, // BVS
            0x90 => snapshot.p & CARRY_BIT == 0,    // BCC
            0xB0 => snapshot.p & CARRY_BIT != 0,    // BCS
            0xD0 => snapshot.p & ZERO_BIT == 0,     // BNE
            0xF0 => snapshot.p & ZERO_BIT != 0,     // BEQ
            _ => false,
        }
    }

    fn is_branch_opcode(opcode: u8) -> bool {
        matches!(
            opcode,
            0x10 | 0x30 | 0x50 | 0x70 | 0x90 | 0xB0 | 0xD0 | 0xF0
        )
    }

    fn abs_x_page_penalty_opcode(opcode: u8) -> bool {
        matches!(
            opcode,
            // Official absolute,X reads
            0x1D | 0x3D | 0x5D | 0x7D | 0xBC | 0xBD | 0xDD | 0xFD
                // Unofficial absolute,X read NOPs
                | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC
        )
    }

    fn abs_y_page_penalty_opcode(opcode: u8) -> bool {
        matches!(
            opcode,
            // Official absolute,Y reads
            0x19 | 0x39 | 0x59 | 0x79 | 0xB9 | 0xBE | 0xD9 | 0xF9
                // Unofficial absolute,Y reads
                | 0xBB | 0xBF
        )
    }

    fn indirect_y_page_penalty_opcode(opcode: u8) -> bool {
        matches!(
            opcode,
            // Official (indirect),Y reads
            0x11 | 0x31 | 0x51 | 0x71 | 0xB1 | 0xD1 | 0xF1
                // Unofficial (indirect),Y reads
                | 0xB3
        )
    }

    /// Swaps all collected CPU writes into the provided vector.
    ///
    /// Note: PRG-space writes (`>= 0x8000`) are also mirrored in
    /// [`swap_prg_writes`](Self::swap_prg_writes) for consumers that need a
    /// dedicated mapper-write stream.
    ///
    /// **Optimization:** Swapping allows the caller to reuse a previously allocated `Vec` capacity,
    /// eliminating continuous heap allocations on the hot path during instruction steps.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::cpu::{Cpu, CpuWrite};
    /// let mut cpu = Cpu::new(0xC000);
    /// let mut buffer = Vec::new();
    /// cpu.swap_writes(&mut buffer);
    /// assert!(buffer.is_empty());
    /// ```
    pub fn swap_writes(&mut self, dest: &mut Vec<CpuWrite>) {
        std::mem::swap(&mut self.writes, dest);
    }

    /// Swaps MMIO read addresses recorded during the last instruction step.
    ///
    /// Always populated regardless of `trace_enabled` — the outer core uses
    /// this to apply MMIO read side-effects ($2002 VBlank clear, $4016 controller
    /// shift, etc.). Swap-reuse eliminates per-step heap allocations.
    pub fn swap_mmio_reads(&mut self, dest: &mut Vec<CpuMmioRead>) {
        std::mem::swap(&mut *self.mmio_reads.borrow_mut(), dest);
    }

    /// Swaps the collected PRG-space writes into the provided vector.
    ///
    /// **Optimization:** Swapping allows the caller to reuse a previously allocated `Vec` capacity,
    /// eliminating continuous heap allocations on the hot path during instruction steps.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::cpu::{Cpu, CpuPrgWrite};
    /// let mut cpu = Cpu::new(0xC000);
    /// let mut buffer = Vec::new();
    /// cpu.swap_prg_writes(&mut buffer);
    /// assert!(buffer.is_empty());
    /// ```
    pub fn swap_prg_writes(&mut self, dest: &mut Vec<CpuPrgWrite>) {
        std::mem::swap(&mut self.prg_writes, dest);
    }

    /// Swaps the bus trace records into the provided vector.
    ///
    /// **Optimization:** Swapping allows the caller to reuse a previously allocated `Vec` capacity,
    /// eliminating continuous heap allocations on the hot path during instruction steps.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::cpu::{Cpu, CpuBusAccess};
    /// let cpu = Cpu::new(0xC000);
    /// let mut buffer = Vec::new();
    /// cpu.swap_bus_trace(&mut buffer);
    /// assert!(buffer.is_empty());
    /// ```
    pub fn swap_bus_trace(&self, dest: &mut Vec<CpuBusAccess>) {
        std::mem::swap(&mut *self.bus_trace.borrow_mut(), dest);
    }

    fn push(&mut self, value: u8) {
        let addr = STACK_BASE.wrapping_add(self.sp as u16);
        self.memory[addr as usize] = value;
        let _ = self.observe_bus_access(addr, value, CpuBusAccessKind::Write);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pull(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        let addr = STACK_BASE.wrapping_add(self.sp as u16);
        self.read(addr)
    }

    /// Formats a trace string only when `trace_enabled`.
    ///
    /// Returns `String::new()` (zero allocation) when tracing is off.
    /// The `format_args!()` captures are already computed by callers for execution
    /// purposes, so LLVM eliminates this call entirely in the disabled branch.
    #[inline(always)]
    fn maybe_trace(
        &self,
        snapshot: TraceSnapshot,
        bytes: &[u8],
        mnemonic: std::fmt::Arguments,
    ) -> String {
        if self.trace_enabled {
            format_trace(snapshot, bytes, mnemonic)
        } else {
            String::new()
        }
    }

    fn observe_bus_access(&self, addr: u16, value: u8, kind: CpuBusAccessKind) -> u8 {
        let next_cycle = self.bus_cycle.get().saturating_add(1);
        self.bus_cycle.set(next_cycle);
        if self.trace_enabled {
            self.bus_trace
                .borrow_mut()
                .push(CpuBusAccess { addr, value, kind });
        }
        next_cycle
    }

    fn pad_microphase_to_cycle_count(&self, pc: u16, cycles: u8) {
        let target = usize::from(cycles);
        let value = self.peek(pc);
        let mut trace = self.bus_trace.borrow_mut();
        while trace.len() < target {
            trace.push(CpuBusAccess {
                addr: pc,
                value,
                kind: CpuBusAccessKind::DummyRead,
            });
        }
    }
}

#[must_use]
fn branch_target(next_pc: u16, offset: u8) -> u16 {
    let signed = i16::from(offset as i8);
    if signed >= 0 {
        next_pc.wrapping_add(signed as u16)
    } else {
        next_pc.wrapping_sub((-signed) as u16)
    }
}

#[must_use]
fn page_crossed(base: u16, indexed: u16) -> bool {
    (base & 0xFF00) != (indexed & 0xFF00)
}

#[must_use]
const fn normalize_cpu_addr(addr: u16) -> u16 {
    match addr {
        // 2KB internal RAM mirrored through $1FFF.
        0x0000..=0x1FFF => addr & 0x07FF,
        // PPU register mirrors every 8 bytes through $3FFF.
        0x2000..=0x3FFF => 0x2000 + ((addr - 0x2000) & 0x0007),
        _ => addr,
    }
}

/// Formats a CPU trace string.
///
/// **Optimization:** Accepts `fmt::Arguments` directly (via `format_args!()`)
/// instead of a pre-formatted string or `&str`. This eliminates intermediate
/// string heap allocations per frame by pre-allocating the required capacity
/// and writing directly into it using `std::fmt::Write`. Manual padding is
/// used to avoid triggering Clippy's `unused_format_specs` lint.
fn format_trace(snapshot: TraceSnapshot, bytes: &[u8], mnemonic: std::fmt::Arguments) -> String {
    use std::fmt::Write;
    let mut result = String::with_capacity(75);
    let _ = write!(&mut result, "{:04X}  ", snapshot.pc);

    let byte_col_start = result.len();
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            let _ = result.write_char(' ');
        }
        let _ = write!(&mut result, "{:02X}", byte);
    }
    while result.len() - byte_col_start < 9 {
        let _ = result.write_char(' ');
    }
    let _ = result.write_char(' ');

    let mnemonic_start = result.len();
    let _ = write!(&mut result, "{}", mnemonic);
    while result.len() - mnemonic_start < 31 {
        let _ = result.write_char(' ');
    }

    let _ = write!(
        &mut result,
        " A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
        snapshot.a, snapshot.x, snapshot.y, snapshot.p, snapshot.sp
    );

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_snapshot_roundtrips_work_ram() {
        let mut cpu = Cpu::new(0xC000);
        cpu.memory[0x0000] = 0xAB;
        cpu.memory[0x00FF] = 0xCD;
        cpu.memory[0x07FF] = 0xEF;

        let snap = cpu.snapshot();
        cpu.memory[0x0000] = 0x00;
        cpu.memory[0x00FF] = 0x00;
        cpu.memory[0x07FF] = 0x00;

        cpu.restore(snap);
        assert_eq!(cpu.memory[0x0000], 0xAB);
        assert_eq!(cpu.memory[0x00FF], 0xCD);
        assert_eq!(cpu.memory[0x07FF], 0xEF);
    }
}

#[cfg(test)]
mod tests_format {
    use super::*;

    #[test]
    fn format_trace_covers_padding_paths() {
        let snapshot = TraceSnapshot {
            pc: 0x8000,
            a: 0x01,
            x: 0x02,
            y: 0x03,
            p: 0x24,
            sp: 0xFD,
        };

        // Short mnemonic to test padding loop
        let out = format_trace(snapshot, &[0xEA], format_args!("NOP"));
        assert!(out.contains("8000"));
        assert!(out.contains("NOP"));

        // Very long mnemonic to test loop
        let long = "THIS_IS_VERY_LONG_MNEMONIC_THAT_WILL_EXCEED_THIRTY_ONE_CHARS";
        let out2 = format_trace(
            snapshot,
            &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
            format_args!("{}", long),
        );
        assert!(out2.contains(long));
    }
}
