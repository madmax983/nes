//! Experimental brute-forcer to find optimal 1-frame inputs.

#[cfg(feature = "nova")]
use crate::{Command, NesCore};

#[cfg(feature = "nova")]
/// Experimental brute-forcer that simulates all possible 1-frame inputs
/// to find the input that maximizes or minimizes a specific memory address.
pub struct InputBruteForcer;

#[cfg(feature = "nova")]
impl InputBruteForcer {
    /// Simulates 1 frame of execution for all 256 possible controller inputs
    /// and returns the input byte that results in the highest value at the target address.
    pub fn maximize_address(core: &NesCore, target_addr: u16) -> u8 {
        let mut best_input = 0;
        let mut max_val = 0;

        for input in 0..=255 {
            let mut sim_core = core.clone();
            let _ = sim_core.execute(Command::SetControllerState(input));
            let _ = sim_core.execute(Command::StepFrame);

            let val = sim_core.read_memory(target_addr);
            if val >= max_val {
                max_val = val;
                best_input = input;
            }
        }

        best_input
    }

    /// Simulates 1 frame of execution for all 256 possible controller inputs
    /// and returns the input byte that results in the lowest value at the target address.
    pub fn minimize_address(core: &NesCore, target_addr: u16) -> u8 {
        let mut best_input = 0;
        let mut min_val = 255;

        for input in 0..=255 {
            let mut sim_core = core.clone();
            let _ = sim_core.execute(Command::SetControllerState(input));
            let _ = sim_core.execute(Command::StepFrame);

            let val = sim_core.read_memory(target_addr);
            if val <= min_val {
                min_val = val;
                best_input = input;
            }
        }

        best_input
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn test_maximize_address() {
        let core = NesCore::new();
        let best_input = InputBruteForcer::maximize_address(&core, 0x0000);
        // Since core does nothing by default, they all result in 0. The loop uses `>=` so the last one (255) wins.
        assert_eq!(best_input, 255);
    }

    #[test]
    fn test_minimize_address() {
        let core = NesCore::new();
        let best_input = InputBruteForcer::minimize_address(&core, 0x0000);
        // Same reasoning, all result in 0, `<=`, so 255 wins.
        assert_eq!(best_input, 255);
    }
}
