//! Time Machine (Nova Experiment)
//!
//! A rewind engine for the NES core, enabling Braid-style time manipulation.

use crate::{CoreSnapshot, NesCore};
use std::collections::VecDeque;

/// A rolling history buffer of emulator states.
#[derive(Debug, Clone)]
pub struct TimeMachine {
    snapshots: VecDeque<CoreSnapshot>,
    max_capacity: usize,
    frames_per_snapshot: u64,
    frames_since_last: u64,
}

impl TimeMachine {
    /// Creates a new TimeMachine.
    #[must_use]
    pub fn new(max_capacity: usize, frames_per_snapshot: u64) -> Self {
        Self {
            snapshots: VecDeque::with_capacity(max_capacity),
            max_capacity,
            frames_per_snapshot,
            frames_since_last: 0,
        }
    }

    /// Ticks the time machine, advancing its internal frame counter.
    pub fn tick(&mut self, core: &NesCore) {
        self.frames_since_last += 1;
        if self.frames_since_last >= self.frames_per_snapshot {
            self.capture(core);
            self.frames_since_last = 0;
        }
    }

    /// Manually captures a snapshot of the current core state.
    pub fn capture(&mut self, core: &NesCore) {
        if self.snapshots.len() >= self.max_capacity {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(core.save_state());
    }

    /// Rewinds the core to the most recent snapshot and removes it from history.
    pub fn rewind(&mut self, core: &mut NesCore) -> bool {
        if let Some(snapshot) = self.snapshots.pop_back() {
            core.load_state(&snapshot);
            self.frames_since_last = 0;
            true
        } else {
            false
        }
    }

    /// Clears the history buffer.
    pub fn clear(&mut self) {
        self.snapshots.clear();
        self.frames_since_last = 0;
    }

    /// Returns the current number of stored snapshots.
    #[must_use]
    pub fn history_len(&self) -> usize {
        self.snapshots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Command;

    #[test]
    fn test_time_machine_rewind() {
        let mut core = NesCore::new();
        // Load a minimal dummy ROM to step correctly
        let mut dummy_rom = vec![
            0x4E, 0x45, 0x53, 0x1A, // "NES\x1A"
            0x01, 0x01, 0x00, 0x00, // 16KB PRG, 8KB CHR, NROM
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Padding
        ];
        dummy_rom.extend(vec![0x00; 16 * 1024 + 8 * 1024]);
        core.load_ines_rom(&dummy_rom).unwrap();

        let mut tm = TimeMachine::new(5, 1);

        // Initial state
        tm.capture(&core);
        assert_eq!(tm.history_len(), 1);

        // Advance state
        core.execute(Command::StepFrame).unwrap();
        tm.capture(&core);
        assert_eq!(tm.history_len(), 2);

        let frames_after = core.ppu_frame_counter();
        assert!(frames_after > 0);

        // Rewind
        let rewound = tm.rewind(&mut core);
        assert!(rewound);
        // Rewind again to get to initial
        let rewound = tm.rewind(&mut core);
        assert!(rewound);

        let frames_before = core.ppu_frame_counter();
        assert_eq!(frames_before, 0);
    }
}
