//! Experimental RAM entropy profiler.
//!
//! This module tracks the frequency of byte values written to each RAM address
//! over time and calculates Shannon entropy. High entropy indicates highly
//! variable data (like RNG or counters), while low entropy indicates static
//! or rarely changing data.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
use std::collections::HashMap;

#[cfg(feature = "nova")]
/// Tracks the frequency of values for each address in the 2KB NES RAM.
pub struct EntropyProfiler {
    /// address -> value -> frequency count
    frequencies: Vec<HashMap<u8, u32>>,
    /// total number of snapshots recorded
    snapshots: u32,
}

#[cfg(feature = "nova")]
impl Default for EntropyProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nova")]
impl EntropyProfiler {
    /// Creates a new, empty entropy profiler.
    #[must_use]
    pub fn new() -> Self {
        Self {
            frequencies: vec![HashMap::new(); 2048],
            snapshots: 0,
        }
    }

    /// Records a snapshot of the current 2KB work RAM.
    pub fn record_ram_snapshot(&mut self, core: &NesCore) {
        let state = core.query(crate::CoreQuery::Registers);
        if let crate::QueryResult::Registers(cpu_state) = state {
            for (addr, &val) in cpu_state.work_ram.iter().enumerate() {
                if addr < 2048 {
                    *self.frequencies[addr].entry(val).or_insert(0) += 1;
                }
            }
            self.snapshots += 1;
        }
    }

    /// Calculates the Shannon entropy for a specific memory address.
    /// Returns a value between 0.0 (static) and 8.0 (completely random).
    #[must_use]
    pub fn calculate_entropy(&self, addr: u16) -> f32 {
        if addr as usize >= self.frequencies.len() || self.snapshots == 0 {
            return 0.0;
        }

        let counts = &self.frequencies[addr as usize];
        let total = self.snapshots as f32;

        let mut entropy = 0.0;
        for &count in counts.values() {
            if count > 0 {
                let p = (count as f32) / total;
                entropy -= p * p.log2();
            }
        }

        entropy
    }

    /// Returns the top `limit` addresses with the highest entropy, sorted descending.
    #[must_use]
    pub fn highest_entropy_addresses(&self, limit: usize) -> Vec<(u16, f32)> {
        let mut results: Vec<(u16, f32)> = (0..2048)
            .map(|addr| (addr, self.calculate_entropy(addr)))
            .collect();

        // Sort descending by entropy, then ascending by address for stability
        results.sort_by(|a, b| {
            b.1.partial_cmp(&a.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.0.cmp(&b.0))
        });

        results.into_iter().take(limit).collect()
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn static_ram_has_zero_entropy() {
        let mut profiler = EntropyProfiler::new();
        let core = NesCore::new(); // Starts with zeroed RAM

        profiler.record_ram_snapshot(&core);
        profiler.record_ram_snapshot(&core);
        profiler.record_ram_snapshot(&core);

        assert_eq!(profiler.calculate_entropy(0), 0.0);
        assert_eq!(profiler.calculate_entropy(100), 0.0);
    }

    #[test]
    fn alternating_ram_has_entropy_one() {
        let mut profiler = EntropyProfiler::new();

        // Simulate alternating values without needing a full NesCore step loop
        profiler.snapshots = 2;
        profiler.frequencies[50].insert(0x00, 1);
        profiler.frequencies[50].insert(0xFF, 1);

        // Entropy for 2 equally probable values should be exactly 1.0 bit
        assert!((profiler.calculate_entropy(50) - 1.0).abs() < 0.001);
    }

    #[test]
    fn highly_random_ram_has_high_entropy() {
        let mut profiler = EntropyProfiler::new();

        profiler.snapshots = 256;
        // Simulate 256 evenly distributed values for address 42
        for i in 0..=255 {
            profiler.frequencies[42].insert(i, 1);
        }

        // Entropy for 256 equally probable values should be exactly 8.0 bits
        assert!((profiler.calculate_entropy(42) - 8.0).abs() < 0.001);
    }

    #[test]
    fn highest_entropy_sorting() {
        let mut profiler = EntropyProfiler::new();
        profiler.snapshots = 4;

        // Addr 1: 0 entropy
        profiler.frequencies[1].insert(0xAA, 4);

        // Addr 2: 1 bit entropy (2 values)
        profiler.frequencies[2].insert(0x01, 2);
        profiler.frequencies[2].insert(0x02, 2);

        // Addr 3: 2 bits entropy (4 values)
        profiler.frequencies[3].insert(0x01, 1);
        profiler.frequencies[3].insert(0x02, 1);
        profiler.frequencies[3].insert(0x03, 1);
        profiler.frequencies[3].insert(0x04, 1);

        let top = profiler.highest_entropy_addresses(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].0, 3); // Addr 3 should be first
        assert_eq!(top[1].0, 2); // Addr 2 should be second
    }
}
