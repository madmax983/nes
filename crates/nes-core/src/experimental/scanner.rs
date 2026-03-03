//! Memory Scanner (Nova Experiment)
//!
//! A cheat-engine style memory scanner for the NES core.
//! Used to track down memory addresses that represent game state (like health, lives, score).

use crate::NesCore;
use std::collections::HashMap;

/// Conditions to filter memory addresses across successive scans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanCondition {
    /// Address value must match this exact u8.
    Exact(u8),
    /// Address value must have decreased since the last scan.
    Decreased,
    /// Address value must have increased since the last scan.
    Increased,
    /// Address value must have changed since the last scan.
    Changed,
    /// Address value must be identical to the last scan.
    Unchanged,
}

/// A tool to scan and filter NES RAM to find specific variables.
#[derive(Debug, Clone, Default)]
pub struct MemoryScanner {
    /// Addresses still considered candidates, mapped to their last known value.
    candidates: HashMap<u16, u8>,
    /// Have we completed the first scan?
    initialized: bool,
}

impl MemoryScanner {
    /// Creates a new, empty memory scanner.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Performs a scan across the main NES RAM ($0000 - $07FF).
    ///
    /// If this is the first scan, it populates the candidate list based on the condition.
    /// If this is a subsequent scan, it filters existing candidates.
    pub fn scan(&mut self, core: &NesCore, condition: ScanCondition) {
        if !self.initialized {
            self.first_scan(core, condition);
            self.initialized = true;
        } else {
            self.subsequent_scan(core, condition);
        }
    }

    /// Resets the scanner, clearing all candidates.
    pub fn reset(&mut self) {
        self.candidates.clear();
        self.initialized = false;
    }

    /// Returns the number of remaining candidate addresses.
    #[must_use]
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// Returns a list of the current candidate addresses.
    #[must_use]
    pub fn candidates(&self) -> Vec<u16> {
        let mut addrs: Vec<u16> = self.candidates.keys().copied().collect();
        addrs.sort_unstable();
        addrs
    }

    fn first_scan(&mut self, core: &NesCore, condition: ScanCondition) {
        for addr in 0x0000..=0x07FF {
            let val = core.read_memory(addr);
            let keep = match condition {
                ScanCondition::Exact(target) => val == target,
                // On first scan, relative conditions just add everything to establish a baseline
                ScanCondition::Decreased
                | ScanCondition::Increased
                | ScanCondition::Changed
                | ScanCondition::Unchanged => true,
            };

            if keep {
                self.candidates.insert(addr, val);
            }
        }
    }

    fn subsequent_scan(&mut self, core: &NesCore, condition: ScanCondition) {
        self.candidates.retain(|&addr, last_val| {
            let current_val = core.read_memory(addr);
            let keep = match condition {
                ScanCondition::Exact(target) => current_val == target,
                ScanCondition::Decreased => current_val < *last_val,
                ScanCondition::Increased => current_val > *last_val,
                ScanCondition::Changed => current_val != *last_val,
                ScanCondition::Unchanged => current_val == *last_val,
            };

            if keep {
                *last_val = current_val; // Update the baseline for the next scan
            }
            keep
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_scanner() {
        let mut core = NesCore::new();
        let mut scanner = MemoryScanner::new();

        // Let's pretend address 0x0042 is player lives, starting at 3
        core.load_cpu_bytes(0x0042, &[3]);
        // And address 0x0100 is score, starting at 0
        core.load_cpu_bytes(0x0100, &[0]);
        // And address 0x0050 is some random changing value
        core.load_cpu_bytes(0x0050, &[10]);

        // First scan: look for exact value 3
        scanner.scan(&core, ScanCondition::Exact(3));

        // At this point, many addresses might be 3, but 0x0042 should be one of them
        assert!(scanner.candidates().contains(&0x0042));
        assert!(!scanner.candidates().contains(&0x0100)); // Score is 0

        // Simulate player losing a life, and random value changing
        core.load_cpu_bytes(0x0042, &[2]);
        core.load_cpu_bytes(0x0050, &[15]);

        // Second scan: we know lives decreased
        scanner.scan(&core, ScanCondition::Decreased);

        // 0x0042 decreased from 3 to 2, so it should still be a candidate
        assert!(scanner.candidates().contains(&0x0042));

        // Simulate another change where lives stays the same, random value changes
        core.load_cpu_bytes(0x0050, &[20]);

        // Third scan: lives should be unchanged
        scanner.scan(&core, ScanCondition::Unchanged);

        // 0x0042 is still 2, so it should be retained
        assert!(scanner.candidates().contains(&0x0042));

        // Let's check Exact again
        scanner.scan(&core, ScanCondition::Exact(2));
        assert!(scanner.candidates().contains(&0x0042));

        // Let's check Changed (it shouldn't be in the list anymore if we look for changed, because it stayed 2)
        // Wait, if it didn't change from last scan, and we filter by Changed, it will be removed.
        scanner.scan(&core, ScanCondition::Changed);
        assert!(!scanner.candidates().contains(&0x0042));
        assert_eq!(scanner.candidate_count(), 0);
    }
}
