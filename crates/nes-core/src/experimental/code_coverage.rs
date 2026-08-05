//! Experimental code coverage tracker for NES ROMs.
//!
//! This module provides the [`CodeCoverageTracker`] utility, allowing developers
//! and reverse engineers to track which CPU addresses have been executed. It
//! builds a map of executed program counters and can export this map as a visual
//! representation to identify hot paths or dead code in the 64KB memory space.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
/// Tracks executed CPU addresses and generates visual coverage maps.
#[derive(Debug, Clone)]
pub struct CodeCoverageTracker {
    coverage: std::vec::Vec<bool>,
    executed_count: usize,
}

#[cfg(feature = "nova")]
impl Default for CodeCoverageTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nova")]
impl CodeCoverageTracker {
    /// Creates a new, empty code coverage tracker.
    #[must_use]
    pub fn new() -> Self {
        Self {
            coverage: vec![false; 65536],
            executed_count: 0,
        }
    }

    /// Records the current CPU Program Counter (PC) as executed.
    pub fn record_execution(&mut self, core: &NesCore) {
        let pc = core.cpu_pc() as usize;
        if !self.coverage[pc] {
            self.coverage[pc] = true;
            self.executed_count += 1;
        }
    }

    /// Returns the number of unique addresses executed so far.
    #[must_use]
    pub fn executed_count(&self) -> usize {
        self.executed_count
    }

    /// Checks if a specific address has been executed.
    #[must_use]
    pub fn was_executed(&self, addr: u16) -> bool {
        self.coverage[addr as usize]
    }

    /// Clears the recorded coverage map.
    pub fn clear(&mut self) {
        self.coverage.fill(false);
        self.executed_count = 0;
    }

    /// Generates a visual 256x256 heatmap of the 64KB address space.
    /// Each pixel represents one byte of address space.
    /// Executed code is bright green, unexecuted is dark gray.
    #[must_use]
    pub fn generate_rgba_visual(&self) -> std::vec::Vec<u8> {
        let mut pixels = vec![0; 65536 * 4];
        for (addr, &executed) in self.coverage.iter().enumerate() {
            let offset = addr * 4;
            if executed {
                pixels[offset] = 0;
                pixels[offset + 1] = 255;
                pixels[offset + 2] = 0;
                pixels[offset + 3] = 255;
            } else {
                pixels[offset] = 30;
                pixels[offset + 1] = 30;
                pixels[offset + 2] = 30;
                pixels[offset + 3] = 255;
            }
        }
        pixels
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_tracking() {
        let mut tracker = CodeCoverageTracker::new();
        assert_eq!(tracker.executed_count(), 0);
        assert!(!tracker.was_executed(0x8000));

        let core = NesCore::new();
        tracker.record_execution(&core);

        assert_eq!(tracker.executed_count(), 1);
        assert!(tracker.was_executed(core.cpu_pc()));

        let visual = tracker.generate_rgba_visual();
        assert_eq!(visual.len(), 65536 * 4);

        tracker.clear();
        assert_eq!(tracker.executed_count(), 0);
        assert!(!tracker.was_executed(core.cpu_pc()));
    }
}
