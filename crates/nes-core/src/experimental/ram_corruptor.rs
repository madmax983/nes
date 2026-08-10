//! Experimental RAM Corruptor.
//!
//! This module provides a utility to corrupt RAM data dynamically
//! to test game robustness or find glitch states.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
pub struct RamCorruptor {
    state: u32,
}

#[cfg(feature = "nova")]
impl RamCorruptor {
    pub fn new(seed: u32) -> Self {
        Self { state: if seed == 0 { 0xDEAD_BEEF } else { seed } }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.state
    }

    pub fn corrupt(&mut self, core: &mut NesCore, intensity_permille: u16) {
        if intensity_permille == 0 { return; }
        let intensity = u32::from(intensity_permille).min(1000);
        for addr in 0x0000..=0x07FF {
            if (self.next_u32() % 1000) < intensity {
                let val = core.read_memory(addr);
                let bit = self.next_u32() % 8;
                core.write_cpu_bus(addr, val ^ (1 << bit));
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;
    #[test]
    fn corrupts_ram() {
        let mut core = NesCore::new();
        let mut corruptor = RamCorruptor::new(1234);
        corruptor.corrupt(&mut core, 500);
    }
}
