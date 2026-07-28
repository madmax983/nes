//! Experimental history tracking for CPU instructions.
//!
//! Tracks a ring buffer of the most recently executed CPU instructions,
//! allowing backwards traversal to see what code led up to a crash or event.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a single CPU instruction execution in history.
pub struct InstructionRecord {
    /// The program counter (PC) where the instruction was read.
    pub pc: u16,
    /// The opcode that was executed.
    pub opcode: u8,
    /// Accumulator register state.
    pub a: u8,
    /// X index register state.
    pub x: u8,
    /// Y index register state.
    pub y: u8,
    /// Processor status register state.
    pub status: u8,
    /// Stack pointer state.
    pub sp: u8,
    /// Total emulator cycle count at execution time.
    pub cycles: u64,
}

#[cfg(feature = "nova")]
/// A ring buffer that tracks the N most recent CPU instructions.
pub struct CpuHistory {
    buffer: Vec<InstructionRecord>,
    capacity: usize,
    head: usize,
    count: usize,
}

#[cfg(feature = "nova")]
impl Default for CpuHistory {
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(feature = "nova")]
impl CpuHistory {
    /// Creates a new CPU history tracker with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![
                InstructionRecord {
                    pc: 0,
                    opcode: 0,
                    a: 0,
                    x: 0,
                    y: 0,
                    status: 0,
                    sp: 0,
                    cycles: 0
                };
                capacity
            ],
            capacity,
            head: 0,
            count: 0,
        }
    }

    /// Records the current CPU state. Should be called before each instruction.
    pub fn record(&mut self, core: &NesCore) {
        if self.capacity == 0 {
            return;
        }

        let pc = core.cpu_pc();
        let opcode = core.read_memory(pc);
        let snapshot = core.cpu_snapshot();

        let record = InstructionRecord {
            pc,
            opcode,
            a: snapshot.a,
            x: snapshot.x,
            y: snapshot.y,
            status: snapshot.status,
            sp: snapshot.sp,
            cycles: core.total_cycles(),
        };

        self.buffer[self.head] = record;
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    /// Retrieves the recorded instruction history in chronological order.
    #[must_use]
    pub fn get_history(&self) -> Vec<InstructionRecord> {
        let mut result = Vec::with_capacity(self.count);
        for i in 0..self.count {
            let index = if self.count < self.capacity {
                i
            } else {
                (self.head + i) % self.capacity
            };
            result.push(self.buffer[index].clone());
        }
        result
    }

    /// Clears the recorded history.
    pub fn clear(&mut self) {
        self.head = 0;
        self.count = 0;
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn history_records_and_wraps() {
        let mut core = NesCore::new();
        let mut history = CpuHistory::new(2);

        history.record(&core);
        core.execute(crate::Command::StepCpu).unwrap();
        history.record(&core);
        core.execute(crate::Command::StepCpu).unwrap();
        history.record(&core);

        let records = history.get_history();
        assert_eq!(records.len(), 2);
    }
}
