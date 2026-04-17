//! Experimental CPU hotspot profiling for NES execution.
//!
//! This module provides the `CpuHotspotProfiler`, which tracks the execution frequency
//! of every instruction in the 64K CPU address space. This is highly useful for optimizing
//! games, understanding busy-wait loops, or generating code execution coverage maps
//! during test runs.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
/// A profiler that counts how many times each address in the CPU memory space is executed.
pub struct CpuHotspotProfiler {
    /// A flat array mapping each of the 65536 possible CPU addresses to an execution count.
    /// This array is heap allocated to prevent massive stack frames but access is O(1).
    execution_counts: Box<[u64; 65536]>,
}

#[cfg(feature = "nova")]
impl Default for CpuHotspotProfiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nova")]
impl CpuHotspotProfiler {
    /// Creates a new, empty hotspot profiler.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::hotspot_profiler::CpuHotspotProfiler;
    /// let profiler = CpuHotspotProfiler::new();
    /// ```
    pub fn new() -> Self {
        // Allocate a boxed array initialized to zero. We use vec! to avoid stack overflow
        // for large arrays and then convert it into a boxed slice, then a box array.
        let counts = vec![0u64; 65536].into_boxed_slice();
        // Safe because length is strictly 65536
        let execution_counts = match counts.try_into() {
            Ok(arr) => arr,
            Err(_) => unreachable!(),
        };

        Self { execution_counts }
    }

    /// Records the execution of the current CPU Program Counter (PC).
    ///
    /// This should be called after every CPU step.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::hotspot_profiler::CpuHotspotProfiler;
    /// # use nes_core::NesCore;
    /// let mut core = NesCore::new();
    /// let mut profiler = CpuHotspotProfiler::new();
    ///
    /// // Track the current PC
    /// profiler.record_step(&core);
    /// assert_eq!(profiler.get_count(core.cpu_pc()), 1);
    /// ```
    pub fn record_step(&mut self, core: &NesCore) {
        let pc = core.cpu_pc() as usize;
        self.execution_counts[pc] = self.execution_counts[pc].saturating_add(1);
    }

    /// Retrieves the execution count for a specific address.
    #[must_use]
    pub fn get_count(&self, addr: u16) -> u64 {
        self.execution_counts[addr as usize]
    }

    /// Clears all recorded execution counts.
    pub fn clear(&mut self) {
        self.execution_counts.fill(0);
    }

    /// Finds the top N most frequently executed addresses.
    ///
    /// Returns a vector of `(address, count)` tuples sorted in descending order by count.
    #[must_use]
    pub fn top_hotspots(&self, n: usize) -> Vec<(u16, u64)> {
        let mut hotspots = Vec::new();
        for (addr, &count) in self.execution_counts.iter().enumerate() {
            if count > 0 {
                hotspots.push((addr as u16, count));
            }
        }
        hotspots.sort_unstable_by_key(|&(_, count)| std::cmp::Reverse(count));
        hotspots.truncate(n);
        hotspots
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn hotspot_profiler_tracks_counts() {
        let mut profiler = CpuHotspotProfiler::new();
        assert_eq!(profiler.get_count(0x8000), 0);

        // Manually increment for testing
        profiler.execution_counts[0x8000] = 5;
        profiler.execution_counts[0x8001] = 10;

        assert_eq!(profiler.get_count(0x8000), 5);
        assert_eq!(profiler.get_count(0x8001), 10);
    }

    #[test]
    fn hotspot_profiler_clears_counts() {
        let mut profiler = CpuHotspotProfiler::new();
        profiler.execution_counts[0x8000] = 5;
        profiler.clear();
        assert_eq!(profiler.get_count(0x8000), 0);
    }

    #[test]
    fn hotspot_profiler_returns_top_hotspots() {
        let mut profiler = CpuHotspotProfiler::new();
        profiler.execution_counts[0x1234] = 10;
        profiler.execution_counts[0x8000] = 50;
        profiler.execution_counts[0xC000] = 100;
        profiler.execution_counts[0x0000] = 5;

        let top = profiler.top_hotspots(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0], (0xC000, 100));
        assert_eq!(top[1], (0x8000, 50));
    }
}
