#![cfg(feature = "nova")]

use crate::NesCore;
use crate::bmp::encode_bmp;
use crate::experimental::event_tracker::EventTracker;

/// Automatically captures a framebuffer screenshot when an EventTracker condition is met.
pub struct AutoClipper {
    tracker: EventTracker,
    last_event_count: usize,
}

impl AutoClipper {
    /// Creates a new `AutoClipper`.
    #[must_use]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            tracker: EventTracker::new(),
            last_event_count: 0,
        }
    }

    /// Adds a memory value condition that should trigger a screenshot.
    pub fn add_condition(&mut self, addr: u16, val: u8) {
        self.tracker.add_trigger(addr, val);
    }

    /// Evaluates current emulator state and returns a BMP screenshot byte vector if a new condition was met.
    ///
    /// `framebuffer` must be exactly 256x240 pixels (245760 bytes) in RGBA format.
    pub fn evaluate(&mut self, core: &NesCore, framebuffer: &[u8]) -> Option<Vec<u8>> {
        self.tracker.track(core);

        let current_event_count = self.tracker.events().len();
        if current_event_count > self.last_event_count {
            self.last_event_count = current_event_count;
            // Capture a screenshot using the raw framebuffer dimensions
            // A standard NES screen is 256x240 pixels.
            return encode_bmp(256, 240, framebuffer).ok();
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Command;

    #[test]
    fn test_autoclipper() {
        let mut core = NesCore::new();
        let mut clipper = AutoClipper::new();

        clipper.add_condition(0x0200, 42);

        let framebuffer = vec![0u8; 256 * 240 * 4];

        let mut result = clipper.evaluate(&core, &framebuffer);
        assert!(result.is_none());

        core.load_cpu_bytes(0x0200, &[42]);
        let _ = core.execute(Command::StepFrame);

        result = clipper.evaluate(&core, &framebuffer);
        assert!(result.is_some());

        let bmp_bytes = result.unwrap();
        assert_eq!(&bmp_bytes[0..2], b"BM");
    }
}
