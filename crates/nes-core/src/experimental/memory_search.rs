#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
pub struct MemorySearcher {
    candidates: Vec<u16>,
    last_snapshot: [u8; 2048],
}

#[cfg(feature = "nova")]
impl MemorySearcher {
    /// Creates a new MemorySearcher, initializing it with all addresses in the 2KB work RAM.
    pub fn new(core: &NesCore) -> Self {
        let snapshot = core.cpu_snapshot().work_ram;
        let candidates = (0..2048).collect();
        Self {
            candidates,
            last_snapshot: snapshot,
        }
    }

    /// Resets the search back to all addresses and updates the baseline snapshot.
    pub fn reset(&mut self, core: &NesCore) {
        self.last_snapshot = core.cpu_snapshot().work_ram;
        self.candidates = (0..2048).collect();
    }

    /// Filters down candidates to only those whose current value exactly matches `target`.
    pub fn filter_eq(&mut self, core: &NesCore, target: u8) {
        let current_ram = core.cpu_snapshot().work_ram;
        self.candidates
            .retain(|&addr| current_ram[addr as usize] == target);
        self.last_snapshot = current_ram;
    }

    /// Filters down candidates to only those whose value has decreased since the last check.
    pub fn filter_decreased(&mut self, core: &NesCore) {
        let current_ram = core.cpu_snapshot().work_ram;
        self.candidates.retain(|&addr| {
            let old_val = self.last_snapshot[addr as usize];
            let new_val = current_ram[addr as usize];
            new_val < old_val
        });
        self.last_snapshot = current_ram;
    }

    /// Filters down candidates to only those whose value has remained unchanged.
    pub fn filter_unchanged(&mut self, core: &NesCore) {
        let current_ram = core.cpu_snapshot().work_ram;
        self.candidates.retain(|&addr| {
            let old_val = self.last_snapshot[addr as usize];
            let new_val = current_ram[addr as usize];
            new_val == old_val
        });
        self.last_snapshot = current_ram;
    }

    /// Returns the current list of candidate addresses and their values.
    pub fn results(&self, core: &NesCore) -> Vec<(u16, u8)> {
        let current_ram = core.cpu_snapshot().work_ram;
        self.candidates
            .iter()
            .map(|&addr| (addr, current_ram[addr as usize]))
            .collect()
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::{Command, NesCore};

    #[test]
    fn memory_search_can_find_exact_value() {
        let mut core = NesCore::new();
        // Load a simple program that writes to $0042
        core.load_cpu_bytes(0xC000, &[0xA9, 0x99, 0x85, 0x42]); // LDA #$99, STA $42

        let mut searcher = MemorySearcher::new(&core);
        assert_eq!(searcher.results(&core).len(), 2048);

        // Execute instructions
        core.execute(Command::StepCpu).unwrap();
        core.execute(Command::StepCpu).unwrap();

        // Search for value 0x99
        searcher.filter_eq(&core, 0x99);
        let results = searcher.results(&core);

        // At least $0042 should contain $99 now
        assert!(results.contains(&(0x0042, 0x99)));
        // Candidates should be drastically reduced
        assert!(results.len() < 2048);
    }

    #[test]
    fn memory_search_can_find_decreased_value() {
        let mut core = NesCore::new();
        // Load a program that decrements $0010
        core.load_cpu_bytes(0xC000, &[0xA9, 0x05, 0x85, 0x10, 0xC6, 0x10]); // LDA #$05, STA $10, DEC $10

        core.execute(Command::StepCpu).unwrap(); // LDA #$05
        core.execute(Command::StepCpu).unwrap(); // STA $10

        let mut searcher = MemorySearcher::new(&core);

        core.execute(Command::StepCpu).unwrap(); // DEC $10

        searcher.filter_decreased(&core);
        let results = searcher.results(&core);

        // $0010 was 5, now 4, so it decreased.
        assert!(results.contains(&(0x0010, 0x04)));
    }
}
