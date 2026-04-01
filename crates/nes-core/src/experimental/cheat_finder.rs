//! Experimental memory scanner for discovering cheat codes and game state values.
//!
//! The cheat finder implements a standard "successive approximation" search, similar
//! to tools found in classic emulators or cheat engines like Game Genie code finders.
//!
//! # The Process
//! 1. **Initialize:** The search begins by snapshotting the entire RAM space (`0x0000` to `0x07FF`).
//!    Every address is a potential candidate.
//! 2. **Play:** The user plays the game to change the state (e.g., losing a life, collecting a coin).
//! 3. **Filter:** The user applies a filter based on what happened (e.g., "value decreased",
//!    "value equals 5"). The finder eliminates addresses that don't match this criteria.
//! 4. **Repeat:** Steps 2 and 3 are repeated until only a handful of candidate addresses remain.
//!    These final addresses are highly likely to be the memory locations controlling the desired state.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a possible memory location that might hold the target cheat value.
///
/// Each candidate tracks both its physical address and the last known value at that address.
/// By storing the `last_value`, subsequent filter operations (like "has changed") can
/// accurately compare the new state against the historical state without needing to
/// buffer the entire RAM.
pub struct MemoryCandidate {
    /// The physical memory address in the NES RAM (typically `0x0000` through `0x07FF`).
    pub addr: u16,
    /// The byte value read at this address during the last filtering pass or initial reset.
    pub last_value: u8,
}

#[cfg(feature = "nova")]
#[derive(Debug, Default, Clone)]
/// The engine for discovering memory locations by repeatedly applying constraints.
///
/// `CheatFinder` maintains an internal list of [`MemoryCandidate`]s. As you filter
/// the candidates against the current state of a [`NesCore`], the list shrinks
/// until only the desired memory address remains.
pub struct CheatFinder {
    candidates: Vec<MemoryCandidate>,
}

#[cfg(feature = "nova")]
impl CheatFinder {
    /// Creates a new, empty `CheatFinder`.
    ///
    /// By default, there are no candidates to search through. You must call
    /// [`CheatFinder::reset_search`] to initialize the candidate list before
    /// performing any filtering operations.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::cheat_finder::CheatFinder;
    /// let finder = CheatFinder::new();
    /// assert_eq!(finder.candidates().len(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
        }
    }

    /// Populates the initial pool of memory candidates using the core's current state.
    ///
    /// The finder loops over the entire base RAM (`0x0000` to `0x07FF`, a total of
    /// 2,048 addresses) and records the current byte values. Any subsequent filtering
    /// applies against this initial historical baseline until updated.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::NesCore;
    /// # use nes_core::experimental::cheat_finder::CheatFinder;
    /// let core = NesCore::new();
    /// let mut finder = CheatFinder::new();
    ///
    /// // Initialize with 2048 candidates
    /// finder.reset_search(&core);
    /// assert_eq!(finder.candidates().len(), 2048);
    /// ```
    pub fn reset_search(&mut self, core: &NesCore) {
        self.candidates.clear();
        for addr in 0x0000..=0x07FF {
            self.candidates.push(MemoryCandidate {
                addr,
                last_value: core.read_memory(addr),
            });
        }
    }

    /// Keeps only candidates whose current memory value matches the exact `value` provided.
    ///
    /// After filtering, the historical `last_value` of the remaining candidates is updated.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::NesCore;
    /// # use nes_core::experimental::cheat_finder::CheatFinder;
    /// let mut core = NesCore::new();
    /// let mut finder = CheatFinder::new();
    /// finder.reset_search(&core);
    ///
    /// // Simulate changing a value at a specific address
    /// core.load_cpu_bytes(0x0123, &[42]);
    ///
    /// // Find the address that now holds the value 42
    /// finder.filter_eq(&core, 42);
    /// assert_eq!(finder.candidates()[0].addr, 0x0123);
    /// ```
    pub fn filter_eq(&mut self, core: &NesCore, value: u8) {
        self.candidates
            .retain(|c| core.read_memory(c.addr) == value);
        self.update_candidates(core);
    }

    /// Keeps only candidates whose current memory value does NOT match the `value` provided.
    ///
    /// After filtering, the historical `last_value` of the remaining candidates is updated.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::NesCore;
    /// # use nes_core::experimental::cheat_finder::CheatFinder;
    /// let core = NesCore::new();
    /// let mut finder = CheatFinder::new();
    /// finder.reset_search(&core);
    ///
    /// // Initially all values are 0, so filtering out 0 leaves nothing.
    /// finder.filter_neq(&core, 0);
    /// assert_eq!(finder.candidates().len(), 0);
    /// ```
    pub fn filter_neq(&mut self, core: &NesCore, value: u8) {
        self.candidates
            .retain(|c| core.read_memory(c.addr) != value);
        self.update_candidates(core);
    }

    /// Keeps only candidates whose memory value has changed since the last filtering operation.
    ///
    /// This is useful when you know a state has changed (e.g., your character moved, took damage)
    /// but you don't know the exact numeric value of that state in memory.
    ///
    /// After filtering, the historical `last_value` of the remaining candidates is updated.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::NesCore;
    /// # use nes_core::experimental::cheat_finder::CheatFinder;
    /// let mut core = NesCore::new();
    /// let mut finder = CheatFinder::new();
    /// finder.reset_search(&core);
    ///
    /// // Simulate an unknown change
    /// core.load_cpu_bytes(0x0200, &[99]);
    ///
    /// // Find all values that changed from their original baseline
    /// finder.filter_changed(&core);
    /// assert_eq!(finder.candidates().len(), 1);
    /// assert_eq!(finder.candidates()[0].addr, 0x0200);
    /// ```
    pub fn filter_changed(&mut self, core: &NesCore) {
        self.candidates
            .retain(|c| core.read_memory(c.addr) != c.last_value);
        self.update_candidates(core);
    }

    /// Keeps only candidates whose memory value has NOT changed since the last filtering operation.
    ///
    /// This is an extremely useful "pruning" mechanism. If your character stands still,
    /// and you filter by `filter_unchanged`, you eliminate memory values related to timers,
    /// enemy movement, rendering counters, and sound generation.
    ///
    /// After filtering, the historical `last_value` of the remaining candidates is updated.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::NesCore;
    /// # use nes_core::experimental::cheat_finder::CheatFinder;
    /// let mut core = NesCore::new();
    /// let mut finder = CheatFinder::new();
    /// finder.reset_search(&core);
    ///
    /// // Simulate time passing (e.g., a frame counter increasing)
    /// core.load_cpu_bytes(0x0111, &[10]);
    ///
    /// // Find values that remained the same (pruning the frame counter)
    /// finder.filter_unchanged(&core);
    /// assert_eq!(finder.candidates().len(), 2047);
    /// ```
    pub fn filter_unchanged(&mut self, core: &NesCore) {
        self.candidates
            .retain(|c| core.read_memory(c.addr) == c.last_value);
        self.update_candidates(core);
    }

    fn update_candidates(&mut self, core: &NesCore) {
        for candidate in &mut self.candidates {
            candidate.last_value = core.read_memory(candidate.addr);
        }
    }

    /// Returns an immutable slice of the currently surviving memory candidates.
    ///
    /// This list shrinks as you apply more filters. The cheat-finding process is typically
    /// considered "complete" when this list reduces down to 1-3 distinct memory locations.
    pub fn candidates(&self) -> &[MemoryCandidate] {
        &self.candidates
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn test_cheat_finder() {
        let mut core = NesCore::new();
        // Set some dummy memory values in RAM space (0x0000 - 0x07FF)
        core.load_cpu_bytes(0x0000, &[0x00, 0x05, 0x0A, 0x05]);

        let mut finder = CheatFinder::new();
        finder.reset_search(&core);
        assert_eq!(finder.candidates().len(), 2048);

        // Filter by exact value 5
        finder.filter_eq(&core, 5);

        // Addresses 0x0001 and 0x0003 should be the only ones remaining
        let candidates = finder.candidates();
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].addr, 0x0001);
        assert_eq!(candidates[1].addr, 0x0003);

        // Change memory
        core.load_cpu_bytes(0x0001, &[0x04]);

        // Filter by changed value
        finder.filter_changed(&core);

        // Only 0x0001 changed
        let candidates = finder.candidates();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].addr, 0x0001);
    }

    #[test]
    fn test_cheat_finder_neq_and_unchanged() {
        let mut core = NesCore::new();
        core.load_cpu_bytes(0x0000, &[0x01, 0x02, 0x03, 0x04]);

        let mut finder = CheatFinder::new();
        finder.reset_search(&core);

        // Filter not equal to 0
        finder.filter_neq(&core, 0);

        // Addresses 0x0000, 0x0001, 0x0002, 0x0003 should remain
        assert_eq!(finder.candidates().len(), 4);

        // Change memory
        core.load_cpu_bytes(0x0001, &[0xFF]);

        // Filter unchanged
        finder.filter_unchanged(&core);

        // Addresses 0x0000, 0x0002, 0x0003 should remain
        let candidates = finder.candidates();
        assert_eq!(candidates.len(), 3);
        assert_eq!(candidates[0].addr, 0x0000);
        assert_eq!(candidates[1].addr, 0x0002);
        assert_eq!(candidates[2].addr, 0x0003);
    }
}
