//! Narrative Generator (Nova Experiment)
//!
//! An experimental feature that watches the NES emulator state and generates
//! a human-readable "story" of what's happening.

use crate::NesCore;

/// A generator that outputs a narrative of the emulator's execution.
pub struct NarrativeGenerator {
    last_pc: u16,
    last_speed: u16,
    last_controller: u8,
}

impl NarrativeGenerator {
    /// Creates a new `NarrativeGenerator`.
    pub fn new() -> Self {
        Self {
            last_pc: 0,
            last_speed: 0,
            last_controller: 0,
        }
    }

    /// Observes the current core state and returns a narrative event if something interesting happened.
    pub fn observe(&mut self, core: &NesCore) -> Option<String> {
        let current_pc = core.cpu_pc();
        let current_speed = core.speed_permille();
        let current_controller = core.controller_bits();

        let mut event = None;

        if self.last_pc == 0 && current_pc != 0 {
            event = Some(format!(
                "The machine powers on. Program Counter is at 0x{:04X}.",
                current_pc
            ));
        } else if self.last_speed != 0 && self.last_speed != current_speed {
            if current_speed > self.last_speed {
                event = Some(format!(
                    "The world accelerates! Speed multiplier set to {} permille.",
                    current_speed
                ));
            } else {
                event = Some(format!(
                    "Time slows down... Speed multiplier set to {} permille.",
                    current_speed
                ));
            }
        } else if self.last_controller != current_controller {
            let newly_pressed = current_controller & !self.last_controller;
            if newly_pressed & crate::api::Button::A.bit_mask() != 0 {
                event = Some("The A button is pressed. The hero jumps!".to_string());
            } else if newly_pressed & crate::api::Button::B.bit_mask() != 0 {
                event = Some("The B button is pressed. An action is taken!".to_string());
            } else if newly_pressed & crate::api::Button::Start.bit_mask() != 0 {
                event = Some("Start is pressed. The journey begins!".to_string());
            }
        }

        self.last_pc = current_pc;
        self.last_speed = current_speed;
        self.last_controller = current_controller;

        event
    }
}

impl Default for NarrativeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Button, Command};

    #[test]
    fn test_narrative_generator() {
        let mut core = NesCore::new();
        let mut generator = NarrativeGenerator::default();

        // 1. Core starts at PC 0xC000
        let event = generator.observe(&core);
        assert_eq!(
            event,
            Some("The machine powers on. Program Counter is at 0xC000.".to_string())
        );

        // 2. No changes, no story
        let event = generator.observe(&core);
        assert_eq!(event, None);

        // 3. Speed changes
        core.execute(Command::SetSpeed(2000)).unwrap();
        let event = generator.observe(&core);
        assert_eq!(
            event,
            Some("The world accelerates! Speed multiplier set to 2000 permille.".to_string())
        );

        // 4. Controller press
        core.execute(Command::PressButton(Button::A)).unwrap();
        let event = generator.observe(&core);
        assert_eq!(
            event,
            Some("The A button is pressed. The hero jumps!".to_string())
        );
    }
}
