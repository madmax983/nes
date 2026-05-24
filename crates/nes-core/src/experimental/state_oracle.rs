#![cfg(feature = "nova")]
//! Experimental State Oracle (Minimal TAS Brute-forcer).
//!
//! The State Oracle clones the emulator state and explores different input branches
//! into the future using Breadth-First Search (BFS). It finds the shortest sequence
//! of inputs required to satisfy a specific memory condition (like increasing a score
//! or altering an RNG seed).

use crate::experimental::event_tracker::Trigger;
use crate::{Button, Command, NesCore};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a possible input for a single frame.
pub enum FrameInput {
    /// Do not press any buttons this frame.
    None,
    /// Press a specific button this frame.
    Button(Button),
}

/// The State Oracle looks into the future to find how to trigger an event.
pub struct StateOracle {
    max_depth: usize,
    allowed_inputs: Vec<FrameInput>,
}

impl StateOracle {
    /// Creates a new `StateOracle`.
    ///
    /// `max_depth` limits how many frames into the future the oracle will explore.
    /// `allowed_inputs` restricts which inputs the oracle is allowed to try,
    /// significantly reducing the branching factor.
    pub fn new(max_depth: usize, allowed_inputs: Vec<FrameInput>) -> Self {
        Self {
            max_depth,
            allowed_inputs,
        }
    }

    /// Explores the future to find the shortest sequence of inputs
    /// that satisfies the `trigger`.
    ///
    /// Returns `Some(Vec<FrameInput>)` if a path is found, or `None` if the
    /// trigger cannot be reached within `max_depth`.
    pub fn find_trigger_path(&self, core: &NesCore, trigger: &Trigger) -> Option<Vec<FrameInput>> {
        let initial_state = core.save_state();

        // Check if we are already at the trigger
        if core.read_memory(trigger.addr) == trigger.expected_value {
            return Some(Vec::new());
        }

        if self.max_depth == 0 || self.allowed_inputs.is_empty() {
            return None;
        }

        // BFS Queue: (State Snapshot, Depth, Input Path)
        let mut queue = VecDeque::new();

        for input in &self.allowed_inputs {
            let mut path = Vec::with_capacity(self.max_depth);
            path.push(input.clone());
            queue.push_back((initial_state.clone(), 1, path));
        }

        let mut temp_core = NesCore::new();

        while let Some((snapshot, depth, path)) = queue.pop_front() {
            // Restore state to this branch
            temp_core.load_state(&snapshot);

            // Apply the last input in the path
            let last_input = path.last().unwrap();
            match last_input {
                FrameInput::None => {
                    let _ = temp_core.execute(Command::SetControllerState(0));
                }
                FrameInput::Button(btn) => {
                    let _ = temp_core.execute(Command::SetControllerState(btn.bit_mask()));
                }
            }

            // Step one frame
            let _ = temp_core.execute(Command::StepFrame);

            // Check trigger
            if temp_core.read_memory(trigger.addr) == trigger.expected_value {
                return Some(path);
            }

            // If we can go deeper, generate new branches from the current new state
            if depth < self.max_depth {
                let new_state = temp_core.save_state();
                for input in &self.allowed_inputs {
                    let mut new_path = path.clone();
                    new_path.push(input.clone());
                    queue.push_back((new_state.clone(), depth + 1, new_path));
                }
            }
        }

        None
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_oracle_finds_path() {
        let mut core = NesCore::new();
        // A dummy ROM or memory setup isn't fully possible here without knowing how CPU steps work.
        // We will mock memory change by having a "fake" execute side-effect.
        // Since we are stepping a frame, if the CPU executes NOPs, nothing changes.
        // But we can't easily inject a game.
        // We'll write a test that fails if we don't handle already-satisfied triggers.

        let trigger = Trigger {
            addr: 0x0200,
            expected_value: 42,
        };
        core.load_cpu_bytes(0x0200, &[42]);

        let oracle = StateOracle::new(3, vec![FrameInput::None, FrameInput::Button(Button::A)]);
        let path = oracle.find_trigger_path(&core, &trigger);
        assert_eq!(path, Some(vec![]));
    }
}
