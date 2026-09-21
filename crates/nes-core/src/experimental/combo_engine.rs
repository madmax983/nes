//! Experimental combo input recognition engine.
//!
//! This module tracks controller inputs over multiple frames and matches them against
//! predefined sequences (combos), enabling fighting-game style inputs or secret cheat codes.

#[cfg(feature = "nova")]
use crate::Button;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Combo {
    pub id: usize,
    pub sequence: Vec<Button>,
    pub max_frames_between_inputs: u32,
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
struct ActiveCombo {
    combo_index: usize,
    sequence_index: usize,
    frames_since_last_input: u32,
}

#[cfg(feature = "nova")]
#[derive(Debug, Default, Clone)]
pub struct ComboEngine {
    combos: Vec<Combo>,
    active_combos: Vec<ActiveCombo>,
    previous_controller_bits: u8,
}

#[cfg(feature = "nova")]
impl ComboEngine {
    pub fn new() -> Self {
        Self {
            combos: Vec::new(),
            active_combos: Vec::new(),
            previous_controller_bits: 0,
        }
    }

    pub fn add_combo(&mut self, id: usize, sequence: Vec<Button>, max_frames_between_inputs: u32) {
        if sequence.is_empty() {
            return;
        }
        self.combos.push(Combo {
            id,
            sequence,
            max_frames_between_inputs,
        });
    }

    pub fn update(&mut self, controller_bits: u8) -> Vec<usize> {
        let mut triggered = Vec::new();
        let pressed_this_frame = controller_bits & !self.previous_controller_bits;

        let mut active_buttons = Vec::new();
        for &button in &[
            Button::A,
            Button::B,
            Button::Select,
            Button::Start,
            Button::Up,
            Button::Down,
            Button::Left,
            Button::Right,
        ] {
            if pressed_this_frame & button.bit_mask() != 0 {
                active_buttons.push(button);
            }
        }

        let mut next_active_combos = Vec::new();

        for active in &mut self.active_combos {
            active.frames_since_last_input += 1;
            let combo = &self.combos[active.combo_index];

            if active.frames_since_last_input > combo.max_frames_between_inputs {
                continue;
            }

            let expected_button = combo.sequence[active.sequence_index];
            if active_buttons.contains(&expected_button) {
                active.sequence_index += 1;
                active.frames_since_last_input = 0;

                if active.sequence_index == combo.sequence.len() {
                    triggered.push(combo.id);
                } else {
                    next_active_combos.push(active.clone());
                }
            } else {
                next_active_combos.push(active.clone());
            }
        }

        for (i, combo) in self.combos.iter().enumerate() {
            let expected_button = combo.sequence[0];
            if active_buttons.contains(&expected_button) {
                if combo.sequence.len() == 1 {
                    triggered.push(combo.id);
                } else {
                    next_active_combos.push(ActiveCombo {
                        combo_index: i,
                        sequence_index: 1,
                        frames_since_last_input: 0,
                    });
                }
            }
        }

        self.active_combos = next_active_combos;
        self.previous_controller_bits = controller_bits;

        triggered
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_combo_recognition() {
        let mut engine = ComboEngine::new();
        engine.add_combo(
            1,
            vec![Button::Up, Button::Up, Button::Down, Button::Down],
            10,
        );

        let mut triggered = Vec::new();
        triggered.extend(engine.update(Button::Up.bit_mask()));
        triggered.extend(engine.update(0));
        triggered.extend(engine.update(Button::Up.bit_mask()));
        triggered.extend(engine.update(0));
        triggered.extend(engine.update(Button::Down.bit_mask()));
        triggered.extend(engine.update(0));
        triggered.extend(engine.update(Button::Down.bit_mask()));

        assert_eq!(triggered, vec![1]);
    }

    #[test]
    fn test_combo_timeout() {
        let mut engine = ComboEngine::new();
        engine.add_combo(1, vec![Button::Up, Button::Up], 2);

        let mut triggered = Vec::new();
        triggered.extend(engine.update(Button::Up.bit_mask()));
        triggered.extend(engine.update(0));
        triggered.extend(engine.update(0));
        triggered.extend(engine.update(0));
        triggered.extend(engine.update(Button::Up.bit_mask()));

        assert!(triggered.is_empty());
    }
}
