//! Experimental memory scanner for discovering cheat codes and game variables.
//!
//! This module provides a `MemoryScanner` that can iteratively filter the NES RAM
//! to locate memory addresses based on value changes over time (e.g., finding where
//! 'lives' or 'health' are stored).

use crate::NesCore;

/// A scanner to search and narrow down memory addresses matching certain criteria.
#[derive(Debug, Clone)]
pub struct MemoryScanner {
    candidates: Vec<(u16, u8)>,
}

impl MemoryScanner {
    /// Initializes the scanner by capturing the current state of a memory range.
    ///
    /// By default, it captures the internal NES CPU RAM (`0x0000..=0x07FF`).
    ///
    /// # Examples
    ///
    /// ```
    /// use nes_core::NesCore;
    /// use nes_core::experimental::scanner::MemoryScanner;
    ///
    /// let core = NesCore::new();
    /// let scanner = MemoryScanner::new(&core);
    /// assert_eq!(scanner.candidates().len(), 2048);
    /// ```
    #[must_use]
    pub fn new(core: &NesCore) -> Self {
        let mut candidates = Vec::with_capacity(2048);
        for addr in 0x0000..=0x07FF {
            candidates.push((addr, core.read_memory(addr)));
        }
        Self { candidates }
    }

    /// Initializes the scanner across a specific custom address range.
    #[must_use]
    pub fn with_range(core: &NesCore, start: u16, end: u16) -> Self {
        let capacity = end.saturating_sub(start).saturating_add(1) as usize;
        let mut candidates = Vec::with_capacity(capacity);
        for addr in start..=end {
            candidates.push((addr, core.read_memory(addr)));
        }
        Self { candidates }
    }

    /// Returns the current list of candidate addresses and their last known values.
    #[must_use]
    pub fn candidates(&self) -> &[(u16, u8)] {
        &self.candidates
    }

    /// Retains only candidates whose current value exactly equals `target`.
    pub fn scan_eq(&mut self, core: &NesCore, target: u8) {
        self.candidates.retain_mut(|(addr, last_val)| {
            let current = core.read_memory(*addr);
            *last_val = current;
            current == target
        });
    }

    /// Retains only candidates whose current value is less than their last known value.
    pub fn scan_decreased(&mut self, core: &NesCore) {
        self.candidates.retain_mut(|(addr, last_val)| {
            let current = core.read_memory(*addr);
            let decreased = current < *last_val;
            *last_val = current;
            decreased
        });
    }

    /// Retains only candidates whose current value is greater than their last known value.
    pub fn scan_increased(&mut self, core: &NesCore) {
        self.candidates.retain_mut(|(addr, last_val)| {
            let current = core.read_memory(*addr);
            let increased = current > *last_val;
            *last_val = current;
            increased
        });
    }

    /// Retains only candidates whose current value is different from their last known value.
    pub fn scan_changed(&mut self, core: &NesCore) {
        self.candidates.retain_mut(|(addr, last_val)| {
            let current = core.read_memory(*addr);
            let changed = current != *last_val;
            *last_val = current;
            changed
        });
    }

    /// Retains only candidates whose current value is exactly the same as their last known value.
    pub fn scan_unchanged(&mut self, core: &NesCore) {
        self.candidates.retain_mut(|(addr, last_val)| {
            let current = core.read_memory(*addr);
            let unchanged = current == *last_val;
            *last_val = current;
            unchanged
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn test_memory_scanner_initialization() {
        let core = NesCore::new();
        let scanner = MemoryScanner::new(&core);
        assert_eq!(scanner.candidates().len(), 2048);

        let custom_scanner = MemoryScanner::with_range(&core, 0x0000, 0x00FF);
        assert_eq!(custom_scanner.candidates().len(), 256);
    }

    #[test]
    fn test_memory_scanner_scan_eq() {
        let mut core = NesCore::new();
        // Load some fake data into RAM
        core.load_cpu_bytes(0x0000, &[0x11, 0x22, 0x33, 0x22, 0x55]);

        let mut scanner = MemoryScanner::with_range(&core, 0x0000, 0x0004);
        assert_eq!(scanner.candidates().len(), 5);

        scanner.scan_eq(&core, 0x22);
        let candidates = scanner.candidates();
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].0, 0x0001);
        assert_eq!(candidates[1].0, 0x0003);
    }

    #[test]
    fn test_memory_scanner_scan_decreased_increased_changed() {
        let mut core = NesCore::new();
        // Initial state
        core.load_cpu_bytes(0x0000, &[10, 20, 30, 40]);
        let scanner = MemoryScanner::with_range(&core, 0x0000, 0x0003);

        // Mutate memory
        core.load_cpu_bytes(0x0000, &[9, 21, 30, 45]);

        // Address 0 decreased, 1 increased, 2 unchanged, 3 increased

        // Clone for testing different scans
        let mut dec_scanner = scanner.clone();
        dec_scanner.scan_decreased(&core);
        assert_eq!(dec_scanner.candidates().len(), 1);
        assert_eq!(dec_scanner.candidates()[0].0, 0x0000);

        let mut inc_scanner = scanner.clone();
        inc_scanner.scan_increased(&core);
        assert_eq!(inc_scanner.candidates().len(), 2);
        assert_eq!(inc_scanner.candidates()[0].0, 0x0001);
        assert_eq!(inc_scanner.candidates()[1].0, 0x0003);

        let mut chg_scanner = scanner.clone();
        chg_scanner.scan_changed(&core);
        assert_eq!(chg_scanner.candidates().len(), 3);
        assert_eq!(chg_scanner.candidates()[0].0, 0x0000);
        assert_eq!(chg_scanner.candidates()[1].0, 0x0001);
        assert_eq!(chg_scanner.candidates()[2].0, 0x0003);

        let mut unchg_scanner = scanner;
        unchg_scanner.scan_unchanged(&core);
        assert_eq!(unchg_scanner.candidates().len(), 1);
        assert_eq!(unchg_scanner.candidates()[0].0, 0x0002);
    }
}
