//! Experimental CPU execution trace buffer.
//!
//! This module provides a ring buffer that records the sequential history of CPU states
//! (Program Counter and registers). This acts as a "flight data recorder", allowing
//! developers to inspect the exact state of the CPU leading up to a breakpoint or crash.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// A snapshot of the CPU registers at a specific point in time.
pub struct TraceEntry {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub status: u8,
}

#[cfg(feature = "nova")]
/// A ring buffer that records the last N CPU states.
pub struct CpuTraceBuffer {
    entries: Vec<TraceEntry>,
    capacity: usize,
    head: usize,
    count: usize,
}

#[cfg(feature = "nova")]
impl CpuTraceBuffer {
    /// Creates a new trace buffer with the specified capacity.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: vec![
                TraceEntry {
                    pc: 0,
                    a: 0,
                    x: 0,
                    y: 0,
                    sp: 0,
                    status: 0
                };
                capacity
            ],
            capacity,
            head: 0,
            count: 0,
        }
    }

    /// Records the current CPU state from the core.
    /// This should be called before or after each CPU step depending on tracking preference.
    pub fn record_step(&mut self, core: &NesCore) {
        if self.capacity == 0 {
            return;
        }
        let snap = core.save_state().cpu;
        let entry = TraceEntry {
            pc: snap.pc,
            a: snap.a,
            x: snap.x,
            y: snap.y,
            sp: snap.sp,
            status: snap.status,
        };

        self.entries[self.head] = entry;
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    /// Retrieves the recorded history in chronological order (oldest to newest).
    #[must_use]
    pub fn history(&self) -> Vec<TraceEntry> {
        let mut result = Vec::with_capacity(self.count);
        if self.count == 0 {
            return result;
        }

        let start_idx = if self.count < self.capacity {
            0
        } else {
            self.head
        };

        for i in 0..self.count {
            let idx = (start_idx + i) % self.capacity;
            result.push(self.entries[idx]);
        }
        result
    }

    /// Clears the recorded trace history.
    pub fn clear(&mut self) {
        self.head = 0;
        self.count = 0;
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn tracks_history_and_wraps_correctly() {
        let mut buffer = CpuTraceBuffer::new(3);
        let mut mock_core = NesCore::new();

        // Step 1
        mock_core.load_cpu_bytes(0x8000, &[0xEA, 0xEA, 0xEA, 0xEA]); // NOP
        buffer.record_step(&mock_core);
        let _ = mock_core.execute(crate::Command::StepCpu);

        // Step 2
        buffer.record_step(&mock_core);
        let _ = mock_core.execute(crate::Command::StepCpu);

        // Step 3
        buffer.record_step(&mock_core);
        let _ = mock_core.execute(crate::Command::StepCpu);

        // Step 4 (should overwrite oldest)
        buffer.record_step(&mock_core);

        let history = buffer.history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].pc, 0x8001); // The second instruction
        assert_eq!(history[2].pc, 0x8003); // The fourth instruction
    }

    #[test]
    fn clear_resets_history() {
        let mut buffer = CpuTraceBuffer::new(5);
        let mock_core = NesCore::new();
        buffer.record_step(&mock_core);
        assert_eq!(buffer.history().len(), 1);

        buffer.clear();
        assert_eq!(buffer.history().len(), 0);
    }
}
