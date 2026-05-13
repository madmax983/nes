//! Experimental TAS brute-forcer for finding optimal input paths.
//!
//! This module provides [`TasBruteForcer`], an experimental tool that performs
//! a breadth-first search over controller inputs to find a sequence of actions
//! that satisfies a specific memory or state condition.

use crate::{Command, NesCore, api::CoreSnapshot};
use std::collections::VecDeque;

/// The result of a successful brute-force search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BruteForceResult {
    /// The sequence of controller inputs (one byte per frame).
    pub inputs: Vec<u8>,
    /// The number of frames simulated to reach the target.
    pub frames_simulated: usize,
}

/// A tool for searching through possible future NES states.
///
/// `TasBruteForcer` uses the emulator's deterministic save states to explore
/// parallel timelines of controller inputs. It is useful for Tool-Assisted
/// Speedruns (TAS) to automatically find the inputs needed to manipulate RNG,
/// maximize speed, or trigger specific game events.
pub struct TasBruteForcer {
    start_state: CoreSnapshot,
    max_depth: u8,
    allowed_inputs: Vec<u8>,
}

impl TasBruteForcer {
    /// Creates a new `TasBruteForcer`.
    ///
    /// * `start_state` - The state to begin branching from.
    /// * `max_depth` - Maximum number of frames to look ahead.
    /// * `allowed_inputs` - The set of controller bitmasks to try each frame.
    #[must_use]
    pub fn new(start_state: CoreSnapshot, max_depth: u8, allowed_inputs: Vec<u8>) -> Self {
        Self {
            start_state,
            max_depth,
            allowed_inputs,
        }
    }

    /// Explores possible inputs until `target_condition` returns `true`.
    ///
    /// Performs a breadth-first search. Returns the shortest sequence of inputs
    /// that satisfies the condition, or `None` if no path is found within `max_depth`.
    pub fn search<F>(&self, core: &mut NesCore, target_condition: F) -> Option<BruteForceResult>
    where
        F: Fn(&NesCore) -> bool,
    {
        if self.max_depth == 0 {
            return None;
        }

        // BFS queue: (depth, input_path, core_snapshot)
        let mut queue: VecDeque<(u8, Vec<u8>, CoreSnapshot)> = VecDeque::new();
        queue.push_back((0, Vec::new(), self.start_state.clone()));

        while let Some((depth, path, state)) = queue.pop_front() {
            if depth >= self.max_depth {
                continue;
            }

            for &input in &self.allowed_inputs {
                core.load_state(&state);
                let _ = core.execute(Command::SetControllerState(input));
                let _ = core.execute(Command::StepFrame);

                let mut new_path = path.clone();
                new_path.push(input);

                if target_condition(core) {
                    return Some(BruteForceResult {
                        inputs: new_path.clone(),
                        frames_simulated: new_path.len(),
                    });
                }

                if depth + 1 < self.max_depth {
                    queue.push_back((depth + 1, new_path, core.save_state()));
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brute_forcer_immediate_match() {
        let mut core = NesCore::new();
        let state = core.save_state();
        let forcer = TasBruteForcer::new(state, 1, vec![0x80]);

        let result = forcer.search(&mut core, |c| c.controller_bits() == 0x80);
        assert!(result.is_some());
        let res = result.unwrap();
        assert_eq!(res.inputs, vec![0x80]);
        assert_eq!(res.frames_simulated, 1);
    }

    #[test]
    fn test_brute_forcer_not_found() {
        let mut core = NesCore::new();
        let state = core.save_state();
        let forcer = TasBruteForcer::new(state, 2, vec![0x00]);

        // Target condition impossible with allowed inputs
        let result = forcer.search(&mut core, |c| c.controller_bits() == 0x40);
        assert!(result.is_none());
    }
}
