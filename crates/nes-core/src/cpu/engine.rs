use core::fmt;

use crate::cpu::status::Status;

const STACK_BASE: u16 = 0x0100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuError {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuSnapshot {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub status: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuWrite {
    pub addr: u16,
    pub value: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuPrgWrite {
    pub addr: u16,
    pub value: u8,
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
}

impl Cpu {
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
            writes: Vec::new(),
            prg_writes: Vec::new(),
        }
    }

    #[must_use]
    pub const fn pc(&self) -> u16 {
        self.pc
    }

    #[must_use]
    pub const fn a(&self) -> u8 {
        self.a
    }

    #[must_use]
    pub const fn x(&self) -> u8 {
        self.x
    }

    #[must_use]
    pub const fn y(&self) -> u8 {
        self.y
    }

    #[must_use]
    pub const fn sp(&self) -> u8 {
        self.sp
    }

    #[must_use]
    pub fn read_byte(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) {
        self.memory[addr as usize] = value;
    }

    #[must_use]
    pub fn snapshot(&self) -> CpuSnapshot {
        CpuSnapshot {
            pc: self.pc,
            a: self.a,
            x: self.x,
            y: self.y,
            sp: self.sp,
            status: self.status.bits(),
        }
    }

    pub fn restore(&mut self, snapshot: CpuSnapshot) {
        self.pc = snapshot.pc;
        self.a = snapshot.a;
        self.x = snapshot.x;
        self.y = snapshot.y;
        self.sp = snapshot.sp;
        self.status = Status::with_bits(snapshot.status);
        self.writes.clear();
        self.prg_writes.clear();
    }

    pub fn reset(&mut self, start_pc: u16) {
        self.pc = start_pc;
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status = Status::with_bits(0x24);
        self.writes.clear();
        self.prg_writes.clear();
    }

    pub fn load_bytes(&mut self, start: u16, bytes: &[u8]) {
        let start = start as usize;
        let end = start.saturating_add(bytes.len()).min(self.memory.len());
        let len = end.saturating_sub(start);
        self.memory[start..end].copy_from_slice(&bytes[..len]);
    }

    pub fn service_nmi(&mut self) {
        let pc = self.pc;
        self.push((pc >> 8) as u8);
        self.push(pc as u8);
        self.push(self.status.bits_for_stack_push());
        self.status.set_interrupt_disable(true);
        self.pc = self.read_u16(0xFFFA);
    }

    pub fn step_with_trace(&mut self) -> Result<String, CpuError> {
        self.writes.clear();
        self.prg_writes.clear();

        let snapshot = TraceSnapshot {
            pc: self.pc,
            a: self.a,
            x: self.x,
            y: self.y,
            p: self.status.bits(),
            sp: self.sp,
        };

        let opcode = self.read(snapshot.pc);
        match opcode {
            0x20 => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let target = u16::from_le_bytes([low, high]);
                let trace = format_trace(
                    snapshot,
                    &[opcode, low, high],
                    &format!("JSR ${target:04X}"),
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
                let trace = format_trace(snapshot, &[opcode, zp], &format!("BIT ${zp:02X}"));
                self.status.update_bit_test(self.a, operand);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x2C => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let operand = self.read(addr);
                let trace =
                    format_trace(snapshot, &[opcode, low, high], &format!("BIT ${addr:04X}"));
                self.status.update_bit_test(self.a, operand);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x4C => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let target = u16::from_le_bytes([low, high]);
                let trace = format_trace(
                    snapshot,
                    &[opcode, low, high],
                    &format!("JMP ${target:04X}"),
                );
                self.pc = target;
                Ok(trace)
            }
            0x60 => {
                let trace = format_trace(snapshot, &[opcode], "RTS");

                let low = self.pull();
                let high = self.pull();
                let return_addr = u16::from_le_bytes([low, high]);
                self.pc = return_addr.wrapping_add(1);
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
                let trace =
                    format_trace(snapshot, &[opcode, low, high], &format!("JMP (${ptr:04X})"));
                self.pc = target;
                Ok(trace)
            }
            0x78 => {
                let trace = format_trace(snapshot, &[opcode], "SEI");
                self.status.set_interrupt_disable(true);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x85 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let trace = format_trace(snapshot, &[opcode, zp], &format!("STA ${zp:02X}"));
                self.write_and_track(zp as u16, self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x86 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let trace = format_trace(snapshot, &[opcode, zp], &format!("STX ${zp:02X}"));
                self.write_and_track(zp as u16, self.x);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x88 => {
                let trace = format_trace(snapshot, &[opcode], "DEY");
                self.y = self.y.wrapping_sub(1);
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x8A => {
                let trace = format_trace(snapshot, &[opcode], "TXA");
                self.a = self.x;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x8D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace =
                    format_trace(snapshot, &[opcode, low, high], &format!("STA ${addr:04X}"));
                self.write_and_track(addr, self.a);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x8E => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace =
                    format_trace(snapshot, &[opcode, low, high], &format!("STX ${addr:04X}"));
                self.write_and_track(addr, self.x);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x90 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace =
                    format_trace(snapshot, &[opcode, offset], &format!("BCC ${target:04X}"));
                self.pc = if !self.status.carry() {
                    target
                } else {
                    next_pc
                };
                Ok(trace)
            }
            0x95 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.x) as u16;
                let trace = format_trace(snapshot, &[opcode, zp], &format!("STA ${zp:02X},X"));
                self.write_and_track(addr, self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x96 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let addr = zp.wrapping_add(self.y) as u16;
                let trace = format_trace(snapshot, &[opcode, zp], &format!("STX ${zp:02X},Y"));
                self.write_and_track(addr, self.x);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0x98 => {
                let trace = format_trace(snapshot, &[opcode], "TYA");
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
                let trace = format_trace(
                    snapshot,
                    &[opcode, low, high],
                    &format!("STA ${base:04X},Y"),
                );
                self.write_and_track(addr, self.a);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0x9A => {
                let trace = format_trace(snapshot, &[opcode], "TXS");
                self.sp = self.x;
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0x9D => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let trace = format_trace(
                    snapshot,
                    &[opcode, low, high],
                    &format!("STA ${base:04X},X"),
                );
                self.write_and_track(addr, self.a);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xA0 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace = format_trace(snapshot, &[opcode, imm], &format!("LDY #${imm:02X}"));
                self.y = imm;
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xA2 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace = format_trace(snapshot, &[opcode, imm], &format!("LDX #${imm:02X}"));
                self.x = imm;
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xA5 => {
                let zp = self.read(snapshot.pc.wrapping_add(1));
                let value = self.read(zp as u16);
                let trace = format_trace(snapshot, &[opcode, zp], &format!("LDA ${zp:02X}"));
                self.a = value;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xA9 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace = format_trace(snapshot, &[opcode, imm], &format!("LDA #${imm:02X}"));
                self.a = imm;
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xAA => {
                let trace = format_trace(snapshot, &[opcode], "TAX");
                self.x = self.a;
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xAC => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace =
                    format_trace(snapshot, &[opcode, low, high], &format!("LDY ${addr:04X}"));
                self.y = self.read(addr);
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xAD => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace =
                    format_trace(snapshot, &[opcode, low, high], &format!("LDA ${addr:04X}"));
                self.a = self.read(addr);
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xAE => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let addr = u16::from_le_bytes([low, high]);
                let trace =
                    format_trace(snapshot, &[opcode, low, high], &format!("LDX ${addr:04X}"));
                self.x = self.read(addr);
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xB0 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace =
                    format_trace(snapshot, &[opcode, offset], &format!("BCS ${target:04X}"));
                self.pc = if self.status.carry() { target } else { next_pc };
                Ok(trace)
            }
            0xBD => {
                let low = self.read(snapshot.pc.wrapping_add(1));
                let high = self.read(snapshot.pc.wrapping_add(2));
                let base = u16::from_le_bytes([low, high]);
                let addr = base.wrapping_add(self.x as u16);
                let trace = format_trace(
                    snapshot,
                    &[opcode, low, high],
                    &format!("LDA ${base:04X},X"),
                );
                self.a = self.read(addr);
                self.status.update_zn(self.a);
                self.pc = self.pc.wrapping_add(3);
                Ok(trace)
            }
            0xC0 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace = format_trace(snapshot, &[opcode, imm], &format!("CPY #${imm:02X}"));
                self.status.update_compare(self.y, imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xC8 => {
                let trace = format_trace(snapshot, &[opcode], "INY");
                self.y = self.y.wrapping_add(1);
                self.status.update_zn(self.y);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xC9 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace = format_trace(snapshot, &[opcode, imm], &format!("CMP #${imm:02X}"));
                self.status.update_compare(self.a, imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xCA => {
                let trace = format_trace(snapshot, &[opcode], "DEX");
                self.x = self.x.wrapping_sub(1);
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xD0 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace =
                    format_trace(snapshot, &[opcode, offset], &format!("BNE ${target:04X}"));
                self.pc = if !self.status.zero() { target } else { next_pc };
                Ok(trace)
            }
            0xD8 => {
                let trace = format_trace(snapshot, &[opcode], "CLD");
                self.status.set_decimal(false);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xE0 => {
                let imm = self.read(snapshot.pc.wrapping_add(1));
                let trace = format_trace(snapshot, &[opcode, imm], &format!("CPX #${imm:02X}"));
                self.status.update_compare(self.x, imm);
                self.pc = self.pc.wrapping_add(2);
                Ok(trace)
            }
            0xE8 => {
                let trace = format_trace(snapshot, &[opcode], "INX");
                self.x = self.x.wrapping_add(1);
                self.status.update_zn(self.x);
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xEA => {
                let trace = format_trace(snapshot, &[opcode], "NOP");
                self.pc = self.pc.wrapping_add(1);
                Ok(trace)
            }
            0xF0 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace =
                    format_trace(snapshot, &[opcode, offset], &format!("BEQ ${target:04X}"));
                self.pc = if self.status.zero() { target } else { next_pc };
                Ok(trace)
            }
            0x10 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace =
                    format_trace(snapshot, &[opcode, offset], &format!("BPL ${target:04X}"));
                self.pc = if !self.status.negative() {
                    target
                } else {
                    next_pc
                };
                Ok(trace)
            }
            0x30 => {
                let offset = self.read(snapshot.pc.wrapping_add(1));
                let next_pc = snapshot.pc.wrapping_add(2);
                let target = branch_target(next_pc, offset);
                let trace =
                    format_trace(snapshot, &[opcode, offset], &format!("BMI ${target:04X}"));
                self.pc = if self.status.negative() {
                    target
                } else {
                    next_pc
                };
                Ok(trace)
            }
            _ => Err(CpuError::UnknownOpcode(opcode)),
        }
    }

    fn read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn read_u16(&self, addr: u16) -> u16 {
        let low = self.read(addr);
        let high = self.read(addr.wrapping_add(1));
        u16::from_le_bytes([low, high])
    }

    fn write_and_track(&mut self, addr: u16, value: u8) {
        self.write_byte(addr, value);
        self.writes.push(CpuWrite { addr, value });
        if addr >= 0x8000 {
            self.prg_writes.push(CpuPrgWrite { addr, value });
        }
    }

    pub fn take_writes(&mut self) -> Vec<CpuWrite> {
        core::mem::take(&mut self.writes)
    }

    pub fn take_prg_writes(&mut self) -> Vec<CpuPrgWrite> {
        core::mem::take(&mut self.prg_writes)
    }

    fn push(&mut self, value: u8) {
        let addr = STACK_BASE.wrapping_add(self.sp as u16);
        self.memory[addr as usize] = value;
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pull(&mut self) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        let addr = STACK_BASE.wrapping_add(self.sp as u16);
        self.memory[addr as usize]
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

fn format_trace(snapshot: TraceSnapshot, bytes: &[u8], mnemonic: &str) -> String {
    let mut byte_col = String::new();
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 {
            byte_col.push(' ');
        }
        byte_col.push_str(&format!("{byte:02X}"));
    }

    format!(
        "{pc:04X}  {byte_col:<9} {mnemonic:<31} A:{a:02X} X:{x:02X} Y:{y:02X} P:{p:02X} SP:{sp:02X}",
        pc = snapshot.pc,
        a = snapshot.a,
        x = snapshot.x,
        y = snapshot.y,
        p = snapshot.p,
        sp = snapshot.sp
    )
}
