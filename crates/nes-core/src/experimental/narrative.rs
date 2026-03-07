//! Narrative Generator (Nova Experiment)
//!
//! A module that watches emulator state (like inputs and memory) over time
//! and generates a human-readable English "story" summarizing the run.
//! This serves as an entertaining alternative to a raw input macro script.

use crate::api::Button;

/// Generates a story based on controller inputs and emulator events.
#[derive(Debug, Clone, Default)]
pub struct NarrativeGenerator {
    active: bool,
    events: Vec<StoryEvent>,
    last_bits: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum StoryEvent {
    Started(u64),
    ButtonPressed(u64, Button),
    ButtonReleased(u64, Button),
    Stopped(u64),
}

impl NarrativeGenerator {
    /// Creates a new, empty narrative generator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts recording events for the story.
    pub fn start(&mut self, current_frame: u64) {
        self.active = true;
        self.events.clear();
        self.events.push(StoryEvent::Started(current_frame));
        self.last_bits = 0;
    }

    /// Stops recording events.
    pub fn stop(&mut self, current_frame: u64) {
        if self.active {
            self.events.push(StoryEvent::Stopped(current_frame));
            self.active = false;
        }
    }

    /// Observes the current controller state. Call this once per frame.
    pub fn observe_frame(&mut self, current_frame: u64, controller_bits: u8) {
        if !self.active {
            return;
        }

        if controller_bits != self.last_bits {
            let pressed = controller_bits & !self.last_bits;
            let released = !controller_bits & self.last_bits;

            self.check_button(current_frame, pressed, released, Button::A);
            self.check_button(current_frame, pressed, released, Button::B);
            self.check_button(current_frame, pressed, released, Button::Select);
            self.check_button(current_frame, pressed, released, Button::Start);
            self.check_button(current_frame, pressed, released, Button::Up);
            self.check_button(current_frame, pressed, released, Button::Down);
            self.check_button(current_frame, pressed, released, Button::Left);
            self.check_button(current_frame, pressed, released, Button::Right);

            self.last_bits = controller_bits;
        }
    }

    fn check_button(&mut self, frame: u64, pressed: u8, released: u8, button: Button) {
        let mask = button.bit_mask();
        if pressed & mask != 0 {
            self.events.push(StoryEvent::ButtonPressed(frame, button));
        } else if released & mask != 0 {
            self.events.push(StoryEvent::ButtonReleased(frame, button));
        }
    }

    /// Generates the English narrative summarizing the recorded events.
    #[must_use]
    pub fn generate_story(&self) -> String {
        if self.events.is_empty() {
            return "Nothing happened.".to_string();
        }

        let mut story = String::new();
        let mut start_frame = 0;

        for event in &self.events {
            match event {
                StoryEvent::Started(f) => {
                    start_frame = *f;
                    story.push_str("Our hero's journey began. ");
                }
                StoryEvent::ButtonPressed(f, b) => {
                    let diff = f.saturating_sub(start_frame);
                    story.push_str(&format!(
                        "After {} frames, they bravely pressed {:?}. ",
                        diff, b
                    ));
                    start_frame = *f; // Reset relative frame counter for the next event
                }
                StoryEvent::ButtonReleased(f, b) => {
                    let diff = f.saturating_sub(start_frame);
                    story.push_str(&format!(
                        "Then, {} frames later, they let go of {:?}. ",
                        diff, b
                    ));
                    start_frame = *f;
                }
                StoryEvent::Stopped(f) => {
                    let diff = f.saturating_sub(start_frame);
                    story.push_str(&format!(
                        "Finally, {} frames passed before they rested.",
                        diff
                    ));
                }
            }
        }

        story.trim_end().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_narrative_generation() {
        let mut generator = NarrativeGenerator::new();
        generator.start(100);

        generator.observe_frame(110, Button::A.bit_mask());
        generator.observe_frame(115, 0); // Released A
        generator.observe_frame(120, Button::Right.bit_mask() | Button::B.bit_mask()); // Pressed Right and B

        generator.stop(130);

        let story = generator.generate_story();
        let expected = "Our hero's journey began. After 10 frames, they bravely pressed A. Then, 5 frames later, they let go of A. After 5 frames, they bravely pressed B. After 0 frames, they bravely pressed Right. Finally, 10 frames passed before they rested.";
        assert_eq!(story, expected);
    }
}
