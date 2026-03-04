//! Macro Recorder (Nova Experiment)
//!
//! A tool that records controller inputs and frame delays from a running NesCore,
//! generating a script compatible with the `nes-mcp` macro engine.

use crate::api::Button;

/// A simple structure to record controller inputs into a macro script string.
#[derive(Debug, Clone, Default)]
pub struct MacroRecorder {
    recording: bool,
    last_bits: u8,
    frames_since_change: u64,
    script: String,
}

impl MacroRecorder {
    /// Creates a new, empty macro recorder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts or resumes recording.
    pub fn start(&mut self) {
        self.recording = true;
    }

    /// Stops recording.
    pub fn stop(&mut self) {
        self.flush_wait();
        self.recording = false;
    }

    /// Returns whether the recorder is currently active.
    #[must_use]
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Returns the accumulated script as a string.
    #[must_use]
    pub fn script(&self) -> &str {
        &self.script
    }

    /// Clears the recorded script.
    pub fn clear(&mut self) {
        self.script.clear();
        self.last_bits = 0;
        self.frames_since_change = 0;
    }

    /// Should be called exactly once per emulator frame (after stepping the frame,
    /// so the inputs applied during that frame are recorded).
    pub fn record_frame(&mut self, controller_bits: u8) {
        if !self.recording {
            return;
        }

        if controller_bits == self.last_bits {
            self.frames_since_change += 1;
            return;
        }

        // State changed. First flush any accumulated WAIT frames.
        self.flush_wait();

        // Figure out which buttons were pressed and released.
        let pressed = controller_bits & !self.last_bits;
        let released = !controller_bits & self.last_bits;

        // Note: the order of checking buttons here determines the output script order when multiple happen in one frame.
        self.check_button_change(pressed, released, Button::A, "A");
        self.check_button_change(pressed, released, Button::B, "B");
        self.check_button_change(pressed, released, Button::Select, "Select");
        self.check_button_change(pressed, released, Button::Start, "Start");
        self.check_button_change(pressed, released, Button::Up, "Up");
        self.check_button_change(pressed, released, Button::Down, "Down");
        self.check_button_change(pressed, released, Button::Left, "Left");
        self.check_button_change(pressed, released, Button::Right, "Right");

        self.last_bits = controller_bits;
        // The frame where the button changed counts as 1 frame of waiting before the *next* change.
        self.frames_since_change = 1;
    }

    fn check_button_change(&mut self, pressed: u8, released: u8, button: Button, name: &str) {
        let mask = button.bit_mask();
        if pressed & mask != 0 {
            self.script.push_str("PRESS ");
            self.script.push_str(name);
            self.script.push('\n');
        } else if released & mask != 0 {
            self.script.push_str("RELEASE ");
            self.script.push_str(name);
            self.script.push('\n');
        }
    }

    fn flush_wait(&mut self) {
        if self.frames_since_change > 0 {
            self.script.push_str("WAIT ");
            // Convert u64 to string using itoa (implicitly via format) or directly.
            // A simple way without allocating extra strings:
            let mut buf = itoa::Buffer::new();
            self.script.push_str(buf.format(self.frames_since_change));
            self.script.push('\n');
            self.frames_since_change = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_recorder_basic() {
        let mut recorder = MacroRecorder::new();
        recorder.start();

        // 3 frames of doing nothing
        recorder.record_frame(0);
        recorder.record_frame(0);
        recorder.record_frame(0);

        // Frame 4: Press A
        recorder.record_frame(Button::A.bit_mask());

        // Frame 5: Hold A
        recorder.record_frame(Button::A.bit_mask());

        // Frame 6: Release A, Press Right
        recorder.record_frame(Button::Right.bit_mask());

        // Stop
        recorder.stop();

        let expected = "WAIT 3\nPRESS A\nWAIT 2\nRELEASE A\nPRESS Right\nWAIT 1\n";
        assert_eq!(recorder.script(), expected);
    }

    #[test]
    fn test_macro_recorder_multiple_buttons() {
        let mut recorder = MacroRecorder::new();
        recorder.start();

        // Press A and B
        recorder.record_frame(Button::A.bit_mask() | Button::B.bit_mask());

        recorder.stop();

        let script = recorder.script();
        assert!(script.contains("PRESS A\n"));
        assert!(script.contains("PRESS B\n"));
        assert!(script.contains("WAIT 1\n"));
        assert!(script.starts_with("PRESS ")); // Wait is printed *after* because state changed on frame 0
    }
}
