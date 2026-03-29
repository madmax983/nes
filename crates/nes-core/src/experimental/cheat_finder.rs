#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryCandidate {
    pub addr: u16,
    pub last_value: u8,
}

#[cfg(feature = "nova")]
#[derive(Debug, Default, Clone)]
pub struct CheatFinder {
    candidates: Vec<MemoryCandidate>,
}

#[cfg(feature = "nova")]
impl CheatFinder {
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
        }
    }

    pub fn reset_search(&mut self, core: &NesCore) {
        self.candidates.clear();
        for addr in 0x0000..=0x07FF {
            self.candidates.push(MemoryCandidate {
                addr,
                last_value: core.read_memory(addr),
            });
        }
    }

    pub fn filter_eq(&mut self, core: &NesCore, value: u8) {
        self.candidates
            .retain(|c| core.read_memory(c.addr) == value);
        self.update_candidates(core);
    }

    pub fn filter_neq(&mut self, core: &NesCore, value: u8) {
        self.candidates
            .retain(|c| core.read_memory(c.addr) != value);
        self.update_candidates(core);
    }

    pub fn filter_changed(&mut self, core: &NesCore) {
        self.candidates
            .retain(|c| core.read_memory(c.addr) != c.last_value);
        self.update_candidates(core);
    }

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
