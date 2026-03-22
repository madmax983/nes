#[cfg(feature = "nova")]
use std::collections::HashMap;

#[cfg(feature = "nova")]
use crate::{CoreQuery, NesCore, QueryResult};

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
pub struct MemoryScanner {
    candidates: HashMap<u16, u8>,
}

#[cfg(feature = "nova")]
impl MemoryScanner {
    pub fn new(core: &NesCore) -> Self {
        let mut candidates = HashMap::new();
        if let QueryResult::Registers(snap) = core.query(CoreQuery::Registers) {
            for (i, &val) in snap.work_ram.iter().enumerate() {
                candidates.insert(i as u16, val);
            }
        }
        Self { candidates }
    }

    pub fn filter_exact(&mut self, core: &NesCore, value: u8) {
        if let QueryResult::Registers(snap) = core.query(CoreQuery::Registers) {
            self.candidates.retain(|&addr, val| {
                let current = snap.work_ram[addr as usize];
                if current == value {
                    *val = current;
                    true
                } else {
                    false
                }
            });
        }
    }

    pub fn filter_decreased(&mut self, core: &NesCore) {
        if let QueryResult::Registers(snap) = core.query(CoreQuery::Registers) {
            self.candidates.retain(|&addr, val| {
                let current = snap.work_ram[addr as usize];
                if current < *val {
                    *val = current;
                    true
                } else {
                    false
                }
            });
        }
    }

    pub fn filter_increased(&mut self, core: &NesCore) {
        if let QueryResult::Registers(snap) = core.query(CoreQuery::Registers) {
            self.candidates.retain(|&addr, val| {
                let current = snap.work_ram[addr as usize];
                if current > *val {
                    *val = current;
                    true
                } else {
                    false
                }
            });
        }
    }

    pub fn filter_changed(&mut self, core: &NesCore) {
        if let QueryResult::Registers(snap) = core.query(CoreQuery::Registers) {
            self.candidates.retain(|&addr, val| {
                let current = snap.work_ram[addr as usize];
                if current != *val {
                    *val = current;
                    true
                } else {
                    false
                }
            });
        }
    }

    pub fn filter_unchanged(&mut self, core: &NesCore) {
        if let QueryResult::Registers(snap) = core.query(CoreQuery::Registers) {
            self.candidates.retain(|&addr, val| {
                let current = snap.work_ram[addr as usize];
                if current == *val {
                    *val = current;
                    true
                } else {
                    false
                }
            });
        }
    }

    pub fn results(&self) -> Vec<u16> {
        let mut addrs: Vec<u16> = self.candidates.keys().copied().collect();
        addrs.sort_unstable();
        addrs
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn test_memory_scanner_exact_and_decreased() {
        let mut core = NesCore::new();
        // Setup initial RAM:
        // Addr 0x10 = 3
        // Addr 0x20 = 3
        // Addr 0x30 = 5
        core.load_cpu_bytes(0x10, &[3]);
        core.load_cpu_bytes(0x20, &[3]);
        core.load_cpu_bytes(0x30, &[5]);

        let mut scanner = MemoryScanner::new(&core);
        scanner.filter_exact(&core, 3);

        let mut res = scanner.results();
        assert!(res.contains(&0x10));
        assert!(res.contains(&0x20));
        assert!(!res.contains(&0x30));

        // Change memory
        // Addr 0x10 = 2 (decreased)
        // Addr 0x20 = 3 (unchanged)
        core.load_cpu_bytes(0x10, &[2]);

        scanner.filter_decreased(&core);
        res = scanner.results();
        assert_eq!(res, vec![0x10]);
    }
}
