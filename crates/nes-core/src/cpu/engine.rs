use crate::cpu::status::Status;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuError {
    UnknownOpcode(u8),
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

    pub fn load_bytes(&mut self, start: u16, bytes: &[u8]) {
        let start = start as usize;
        let end = start.saturating_add(bytes.len()).min(self.memory.len());
        let len = end.saturating_sub(start);
        self.memory[start..end].copy_from_slice(&bytes[..len]);
    }

    pub fn step_with_trace(&mut self) -> Result<String, CpuError> {
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
            _ => Err(CpuError::UnknownOpcode(opcode)),
        }
    }

    fn read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
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
