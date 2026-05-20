//! Experimental automated camera that captures screenshots triggered by memory events.
//!
//! This module combines the `EventTracker` and framebuffer capabilities to
//! automatically snap pictures when specific in-game events occur (e.g., leveling up,
//! reaching a high score, or dying).

#![cfg(feature = "nova")]

use crate::NesCore;
use crate::bmp::encode_bmp;
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};
use crate::experimental::event_tracker::EventTracker;

/// A photograph taken automatically when a memory condition is met.
#[derive(Debug, Clone)]
pub struct EventPhoto {
    /// The memory address that triggered the photo.
    pub trigger_addr: u16,
    /// The memory value that triggered the photo.
    pub trigger_value: u8,
    /// The BMP encoded image bytes.
    pub bmp_data: Vec<u8>,
}

/// Automatically takes screenshots when specific memory conditions are met.
#[derive(Debug, Default, Clone)]
pub struct EventCamera {
    tracker: EventTracker,
    photos: Vec<EventPhoto>,
    last_event_count: usize,
}

impl EventCamera {
    /// Creates a new, empty `EventCamera`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tracker: EventTracker::new(),
            photos: Vec::new(),
            last_event_count: 0,
        }
    }

    /// Instructs the camera to take a photo when `addr` equals `expected_value`.
    pub fn add_trigger(&mut self, addr: u16, expected_value: u8) {
        self.tracker.add_trigger(addr, expected_value);
    }

    /// Evaluates memory and takes a photo if any triggers matched.
    pub fn track(&mut self, core: &NesCore) {
        self.tracker.track(core);
        let current_count = self.tracker.events().len();

        if current_count > self.last_event_count {
            let rgba = core.framebuffer_rgba();
            if let Ok(bmp_data) = encode_bmp(FRAME_WIDTH, FRAME_HEIGHT, &rgba) {
                for event in &self.tracker.events()[self.last_event_count..current_count] {
                    self.photos.push(EventPhoto {
                        trigger_addr: event.addr,
                        trigger_value: event.value,
                        bmp_data: bmp_data.clone(),
                    });
                }
            }
            self.last_event_count = current_count;
        }
    }

    /// Returns all captured photos.
    #[must_use]
    pub fn photos(&self) -> &[EventPhoto] {
        &self.photos
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_event_camera() {
        let mut core = NesCore::new();
        let mut camera = EventCamera::new();
        camera.add_trigger(0x0100, 0x55);

        camera.track(&core);
        assert_eq!(camera.photos().len(), 0, "No trigger yet");

        core.load_cpu_bytes(0x0100, &[0x55]);
        camera.track(&core);

        assert_eq!(camera.photos().len(), 1, "Should have taken a photo");
        let photo = &camera.photos()[0];
        assert_eq!(photo.trigger_addr, 0x0100);
        assert_eq!(photo.trigger_value, 0x55);
        assert_eq!(&photo.bmp_data[0..2], b"BM", "Must be a valid BMP");
    }
}
