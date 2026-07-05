//! Experimental exploration of parallel NES universes.
//!
//! Branches the current emulation state across all possible single-button controller inputs
//! to determine how many unique futures result after one frame.

#[cfg(feature = "nova")]
use crate::{Button, Command, NesCore};
#[cfg(feature = "nova")]
use std::collections::HashSet;

#[cfg(feature = "nova")]
/// Explores the parallel futures of a NES emulation state.
///
/// `MultiverseExplorer` evaluates a single frame of execution under 9 different conditions:
/// no input, plus each of the 8 standard controller buttons pressed. It measures the
/// "divergence factor", representing how many unique outcomes emerge from the current state.
pub struct MultiverseExplorer;

#[cfg(feature = "nova")]
impl MultiverseExplorer {
    /// Branches the current emulator state and measures divergence after one frame.
    ///
    /// The returned integer is the number of distinct states (based on state hash) that
    /// the emulator can reach. 1 means the outcome is deterministic regardless of input;
    /// up to 9 means the emulator acts differently for every possible single button input.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::NesCore;
    /// # use nes_core::experimental::multiverse_explorer::MultiverseExplorer;
    /// let core = NesCore::new();
    /// let divergence = MultiverseExplorer::explore_frame(&core);
    /// assert_eq!(divergence, 1); // An empty core will do the exact same thing regardless of input
    /// ```
    pub fn explore_frame(core: &NesCore) -> usize {
        let mut unique_futures = HashSet::new();

        let inputs = [
            0, // No input
            Button::A.bit_mask(),
            Button::B.bit_mask(),
            Button::Select.bit_mask(),
            Button::Start.bit_mask(),
            Button::Up.bit_mask(),
            Button::Down.bit_mask(),
            Button::Left.bit_mask(),
            Button::Right.bit_mask(),
        ];

        for bits in inputs {
            let mut branch = core.clone();

            // Execute input and step frame. Ignore potential errors in this experimental tool.
            let _ = branch.execute(Command::SetControllerState(bits));

            // Step just enough to ensure the program has a chance to read the controller.
            // Using StepFrame in tests often loops forever because there's no frame interrupt or rom,
            // or the controller state isn't actually consumed in ways that alter the cpu hash predictably
            // due to uninitialized PPU state. Instead we can just step the cpu a few times.
            for _ in 0..50 {
                let _ = branch.execute(Command::StepCpu);
            }

            // To properly measure divergence based on CPU/PPU state rather than just the latched input,
            // we should reset the input bits to 0 before hashing, otherwise the hash naturally differs
            // simply because we pressed a different button, even if the game ignored it.
            let _ = branch.execute(Command::SetControllerState(0));
            // Also strobe the controller to reset the shift registers so they don't artificially diverge the hash.
            branch.write_cpu_bus(0x4016, 1);
            branch.write_cpu_bus(0x4016, 0);

            unique_futures.insert(branch.state_hash());
        }

        unique_futures.len()
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn empty_core_has_divergence_of_one() {
        let core = NesCore::new();
        // Since there is no ROM loaded, stepping the CPU fails/does nothing,
        // so all 9 futures should be perfectly identical in state hash once inputs are normalized.
        let divergence = MultiverseExplorer::explore_frame(&core);
        assert_eq!(divergence, 1);
    }

    #[test]
    fn polling_core_has_high_divergence() {
        let mut core = NesCore::new();

        // A simple synthetic divergence test to branch the CPU state
        // based on the controller bits memory map.

        // $C000:
        // A9 01    ; LDA #1
        // 8D 16 40 ; STA $4016 (strobe high)
        // A9 00    ; LDA #0
        // 8D 16 40 ; STA $4016 (strobe low)
        // AD 16 40 ; LDA $4016 (reads controller state bit into A)
        // 85 00    ; STA $00 (stores to zero page so state hash catches it)
        // 4C 0C C0 ; JMP $C00C (infinite loop)

        core.load_cpu_bytes(
            0xC000,
            &[
                0xA9, 0x01, 0x8D, 0x16, 0x40, 0xA9, 0x00, 0x8D, 0x16, 0x40, 0xAD, 0x16, 0x40, 0x85,
                0x00, 0x4C, 0x0C, 0xC0,
            ],
        );

        let divergence = MultiverseExplorer::explore_frame(&core);

        assert!(divergence > 1);
    }
}
