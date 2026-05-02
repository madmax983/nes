//! Experimental memory entropy analyzer for identifying volatile RAM.
//!
//! This module provides the `RamEntropyAnalyzer`, which computes the Shannon entropy
//! of values at each RAM address over time. This is useful for identifying
//! RNG seeds, frame counters, or other volatile memory locations.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
/// Analyzes the entropy of NES RAM to find volatile memory locations.
///
/// `RamEntropyAnalyzer` tracks the frequency of byte values across the 2KB internal RAM
/// (addresses `0x0000` through `0x07FF`). By calculating the Shannon entropy of these
/// frequencies, it can identify highly volatile memory like RNG seeds or frequently
/// changing state variables.
pub struct RamEntropyAnalyzer {
    /// Tracks the frequency of each byte value (0-255) for each RAM address (0-2047).
    frequencies: Box<[[u64; 256]; 2048]>,
    /// The total number of snapshots recorded.
    total_snapshots: u64,
}

#[cfg(feature = "nova")]
impl Default for RamEntropyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nova")]
impl RamEntropyAnalyzer {
    /// Creates a new `RamEntropyAnalyzer` with all frequencies initialized to zero.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::ram_entropy::RamEntropyAnalyzer;
    /// let analyzer = RamEntropyAnalyzer::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        // Use vec! to allocate on the heap and avoid blowing up the stack
        let counts = vec![[0u64; 256]; 2048].into_boxed_slice();
        let frequencies = match counts.try_into() {
            Ok(arr) => arr,
            Err(_) => unreachable!(),
        };

        Self {
            frequencies,
            total_snapshots: 0,
        }
    }

    /// Records the current state of the NES RAM.
    ///
    /// This should be called periodically (e.g., once per frame) to build a history
    /// of byte frequencies.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::ram_entropy::RamEntropyAnalyzer;
    /// # use nes_core::NesCore;
    /// let core = NesCore::new();
    /// let mut analyzer = RamEntropyAnalyzer::new();
    ///
    /// // Record the current RAM state
    /// analyzer.record_snapshot(&core);
    /// ```
    pub fn record_snapshot(&mut self, core: &NesCore) {
        for addr in 0..2048 {
            let value = core.read_memory(addr as u16);
            self.frequencies[addr][value as usize] =
                self.frequencies[addr][value as usize].saturating_add(1);
        }
        self.total_snapshots = self.total_snapshots.saturating_add(1);
    }

    /// Calculates the Shannon entropy for a specific RAM address (`0x0000` to `0x07FF`).
    ///
    /// Entropy ranges from 0.0 (the value never changed) to 8.0 (every possible byte
    /// value occurred with equal frequency).
    #[must_use]
    pub fn entropy(&self, addr: u16) -> f64 {
        if self.total_snapshots == 0 || addr >= 2048 {
            return 0.0;
        }

        let mut entropy = 0.0;
        let total = self.total_snapshots as f64;

        for &count in self.frequencies[addr as usize].iter() {
            if count > 0 {
                let p = (count as f64) / total;
                entropy -= p * p.log2();
            }
        }

        entropy
    }

    /// Finds the top N most volatile RAM addresses based on their entropy.
    ///
    /// Returns a vector of `(address, entropy)` tuples sorted in descending order by entropy.
    #[must_use]
    pub fn top_volatile(&self, n: usize) -> Vec<(u16, f64)> {
        let mut results = Vec::with_capacity(2048);
        for addr in 0..2048 {
            let e = self.entropy(addr as u16);
            if e > 0.0 {
                results.push((addr as u16, e));
            }
        }

        // Sort by entropy descending
        results.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(n);
        results
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn ram_entropy_static_is_zero() {
        let mut analyzer = RamEntropyAnalyzer::new();

        // Address 0x0000 is constantly 0x42
        for _ in 0..100 {
            analyzer.frequencies[0][0x42] += 1;
            analyzer.total_snapshots += 1;
        }

        assert_eq!(analyzer.entropy(0x0000), 0.0);
    }

    #[test]
    fn ram_entropy_uniform_is_eight() {
        let mut analyzer = RamEntropyAnalyzer::new();

        // Address 0x0010 has every value exactly once
        for val in 0..=255 {
            analyzer.frequencies[0x10][val as usize] += 1;
            analyzer.total_snapshots += 1;
        }

        // Entropy of uniform distribution over 256 values should be log2(256) = 8.0
        let e = analyzer.entropy(0x0010);
        assert!((e - 8.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ram_entropy_top_volatile_sorting() {
        let mut analyzer = RamEntropyAnalyzer::new();

        // 0x0000 -> static (0.0 entropy)
        analyzer.frequencies[0x0000][1] = 10;

        // 0x0001 -> 2 values evenly (1.0 entropy)
        analyzer.frequencies[0x0001][1] = 5;
        analyzer.frequencies[0x0001][2] = 5;

        // 0x0002 -> 4 values evenly (2.0 entropy)
        analyzer.frequencies[0x0002][1] = 2;
        analyzer.frequencies[0x0002][2] = 2;
        analyzer.frequencies[0x0002][3] = 3;
        analyzer.frequencies[0x0002][4] = 3;

        analyzer.total_snapshots = 10;

        let top = analyzer.top_volatile(3);
        assert_eq!(top.len(), 2); // only > 0.0
        assert_eq!(top[0].0, 0x0002);
        assert_eq!(top[1].0, 0x0001);
    }
}
