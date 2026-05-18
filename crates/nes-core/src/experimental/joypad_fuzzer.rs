//! Experimental tools for generating deterministic, randomized joypad inputs.
//!
//! This module provides the `JoypadFuzzer`, which generates streams of random
//! controller inputs. This is useful for chaos engineering, fuzzing the emulator
//! to find crashes, or testing how games handle impossible inputs (like Left + Right).

#[cfg(feature = "nova")]
use crate::api::Button;

#[cfg(feature = "nova")]
/// A deterministic input generator for NES controllers.
pub struct JoypadFuzzer {
    state: u32,
}

#[cfg(feature = "nova")]
impl JoypadFuzzer {
    /// Creates a new `JoypadFuzzer` seeded with a specific value.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
        }
    }

    /// Advances the internal LCG state and returns a pseudo-random `u32`.
    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.state
    }

    /// Generates a random controller bitmask.
    ///
    /// If `allow_impossible` is false, it sanitizes the inputs to prevent
    /// physically impossible combinations (Up + Down, Left + Right) from
    /// being pressed simultaneously.
    pub fn next_input(&mut self, allow_impossible: bool) -> u8 {
        let mut input = (self.next_u32() & 0xFF) as u8;

        if !allow_impossible {
            let up = Button::Up.bit_mask();
            let down = Button::Down.bit_mask();
            let left = Button::Left.bit_mask();
            let right = Button::Right.bit_mask();

            if (input & (up | down)) == (up | down) {
                // If both Up and Down are pressed, randomly clear one
                if self.next_u32().rem_euclid(2) == 0 {
                    input &= !up;
                } else {
                    input &= !down;
                }
            }

            if (input & (left | right)) == (left | right) {
                // If both Left and Right are pressed, randomly clear one
                if self.next_u32().rem_euclid(2) == 0 {
                    input &= !left;
                } else {
                    input &= !right;
                }
            }
        }

        input
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn joypad_fuzzer_is_deterministic() {
        let mut fuzzer1 = JoypadFuzzer::new(42);
        let mut fuzzer2 = JoypadFuzzer::new(42);

        for _ in 0..100 {
            assert_eq!(fuzzer1.next_input(true), fuzzer2.next_input(true));
        }
    }

    #[test]
    fn joypad_fuzzer_prevents_impossible_inputs() {
        let mut fuzzer = JoypadFuzzer::new(1337);

        let up = Button::Up.bit_mask();
        let down = Button::Down.bit_mask();
        let left = Button::Left.bit_mask();
        let right = Button::Right.bit_mask();

        for _ in 0..1000 {
            let input = fuzzer.next_input(false);
            assert_ne!((input & (up | down)), (up | down));
            assert_ne!((input & (left | right)), (left | right));
        }
    }

    #[test]
    fn joypad_fuzzer_allows_impossible_inputs_when_flag_is_set() {
        let mut fuzzer = JoypadFuzzer::new(777);

        let up = Button::Up.bit_mask();
        let down = Button::Down.bit_mask();
        let left = Button::Left.bit_mask();
        let right = Button::Right.bit_mask();

        let mut found_impossible_y = false;
        let mut found_impossible_x = false;

        for _ in 0..1000 {
            let input = fuzzer.next_input(true);
            if (input & (up | down)) == (up | down) {
                found_impossible_y = true;
            }
            if (input & (left | right)) == (left | right) {
                found_impossible_x = true;
            }
        }

        assert!(found_impossible_y);
        assert!(found_impossible_x);
    }
}
