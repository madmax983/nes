//! Experimental visual heatmap generator for NES framebuffers.
//!
//! This module provides `VisualHeatmap`, a tool that monitors the visual output of the NES over time.
//! By comparing consecutive frames, it builds a cumulative "heatmap" highlighting which pixels
//! change the most frequently. This is useful for identifying dynamic elements, UI bounding boxes,
//! or simply generating cool visualizations of "action zones" in a game.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH, FRAME_RGBA_BYTES};

#[cfg(feature = "nova")]
/// Generates a heatmap based on pixel change frequency over multiple frames.
pub struct VisualHeatmap {
    /// Tracks the number of times each pixel (ignoring RGBA separation) has changed.
    change_counts: Box<[u32; FRAME_WIDTH * FRAME_HEIGHT]>,
    /// The RGBA buffer of the previously tracked frame.
    last_frame: Box<[u8; FRAME_RGBA_BYTES]>,
    /// The maximum change count observed, used for normalizing the heatmap gradient.
    max_count: u32,
    /// Has a frame been tracked yet?
    has_last_frame: bool,
}

#[cfg(feature = "nova")]
impl Default for VisualHeatmap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nova")]
impl VisualHeatmap {
    /// Creates a new, empty visual heatmap.
    pub fn new() -> Self {
        let change_counts = vec![0u32; FRAME_WIDTH * FRAME_HEIGHT].into_boxed_slice();
        let change_counts = match change_counts.try_into() {
            Ok(arr) => arr,
            Err(_) => unreachable!(),
        };

        let last_frame = vec![0u8; FRAME_RGBA_BYTES].into_boxed_slice();
        let last_frame = match last_frame.try_into() {
            Ok(arr) => arr,
            Err(_) => unreachable!(),
        };

        Self {
            change_counts,
            last_frame,
            max_count: 0,
            has_last_frame: false,
        }
    }

    /// Tracks a new frame, comparing it to the previously tracked frame.
    ///
    /// Any pixels that differ between the current frame and the last frame will have
    /// their change counts incremented.
    ///
    /// The provided `frame` must be an RGBA8 buffer of size `FRAME_RGBA_BYTES`.
    pub fn track_frame(&mut self, frame: &[u8]) {
        if frame.len() != FRAME_RGBA_BYTES {
            return;
        }

        if !self.has_last_frame {
            self.last_frame.copy_from_slice(frame);
            self.has_last_frame = true;
            return;
        }

        for i in 0..(FRAME_WIDTH * FRAME_HEIGHT) {
            let px_idx = i * 4;
            // A pixel has changed if any of its RGB components have changed.
            // (We ignore alpha since it's typically constant 255).
            let changed = self.last_frame[px_idx] != frame[px_idx] ||
                          self.last_frame[px_idx + 1] != frame[px_idx + 1] ||
                          self.last_frame[px_idx + 2] != frame[px_idx + 2];

            if changed {
                self.change_counts[i] = self.change_counts[i].saturating_add(1);
                if self.change_counts[i] > self.max_count {
                    self.max_count = self.change_counts[i];
                }
            }
        }

        self.last_frame.copy_from_slice(frame);
    }

    /// Renders the cumulative heatmap into an RGBA8 buffer.
    ///
    /// The resulting image uses a cool-to-warm gradient:
    /// - Unchanged pixels are Dark Blue.
    /// - Infrequently changed pixels transition to Light Blue/Cyan.
    /// - Moderately changed pixels transition to Yellow.
    /// - Highly changed pixels transition to Red.
    ///
    /// The output `out_frame` must be an RGBA8 buffer of size `FRAME_RGBA_BYTES`.
    pub fn render_heatmap(&self, out_frame: &mut [u8]) {
        if out_frame.len() != FRAME_RGBA_BYTES {
            return;
        }

        for i in 0..(FRAME_WIDTH * FRAME_HEIGHT) {
            let px_idx = i * 4;
            let count = self.change_counts[i];

            // Normalize intensity between 0.0 and 1.0
            let intensity = if self.max_count == 0 {
                0.0
            } else {
                count as f32 / self.max_count as f32
            };

            let (r, g, b) = Self::intensity_to_rgb(intensity);

            out_frame[px_idx] = r;
            out_frame[px_idx + 1] = g;
            out_frame[px_idx + 2] = b;
            out_frame[px_idx + 3] = 255;
        }
    }

    /// Clears the heatmap data and resets the tracker.
    pub fn clear(&mut self) {
        self.change_counts.fill(0);
        self.last_frame.fill(0);
        self.max_count = 0;
        self.has_last_frame = false;
    }

    /// Maps a normalized intensity [0.0, 1.0] to a Blue -> Cyan -> Yellow -> Red gradient.
    fn intensity_to_rgb(intensity: f32) -> (u8, u8, u8) {
        let i = intensity.clamp(0.0, 1.0);

        if i < 0.33 {
            // Blue to Cyan
            let t = i / 0.33;
            (0, (255.0 * t) as u8, 255)
        } else if i < 0.66 {
            // Cyan to Yellow
            let t = (i - 0.33) / 0.33;
            ((255.0 * t) as u8, 255, (255.0 * (1.0 - t)) as u8)
        } else {
            // Yellow to Red
            let t = (i - 0.66) / 0.34;
            (255, (255.0 * (1.0 - t)) as u8, 0)
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn tracks_pixel_changes_correctly() {
        let mut heatmap = VisualHeatmap::new();
        let frame1 = vec![0u8; FRAME_RGBA_BYTES];
        let mut frame2 = vec![0u8; FRAME_RGBA_BYTES];

        // Frame 1: all black
        heatmap.track_frame(&frame1);

        // Frame 2: Top-left pixel becomes white
        frame2[0] = 255;
        frame2[1] = 255;
        frame2[2] = 255;
        heatmap.track_frame(&frame2);

        assert_eq!(heatmap.change_counts[0], 1); // Pixel 0 changed
        assert_eq!(heatmap.change_counts[1], 0); // Pixel 1 unchanged
        assert_eq!(heatmap.max_count, 1);
    }

    #[test]
    fn renders_heatmap_gradient() {
        let mut heatmap = VisualHeatmap::new();
        let frame1 = vec![0u8; FRAME_RGBA_BYTES];
        let mut frame2 = vec![0u8; FRAME_RGBA_BYTES];
        let mut frame3 = vec![0u8; FRAME_RGBA_BYTES];

        // Track changes to create varying intensities
        heatmap.track_frame(&frame1);

        // Change Pixel 0
        frame2[0] = 255;
        heatmap.track_frame(&frame2);

        // Change Pixel 0 again, and Pixel 1
        frame3[0] = 0;
        frame3[4] = 255; // Pixel 1
        heatmap.track_frame(&frame3);

        // Pixel 0 changed twice (intensity 1.0 - Red)
        // Pixel 1 changed once (intensity 0.5 - Yellow-ish)
        // Pixel 2 never changed (intensity 0.0 - Blue)

        let mut out_frame = vec![0u8; FRAME_RGBA_BYTES];
        heatmap.render_heatmap(&mut out_frame);

        // Pixel 0 should be Red
        assert_eq!(out_frame[0], 255); // R
        assert_eq!(out_frame[1], 0);   // G
        assert_eq!(out_frame[2], 0);   // B

        // Pixel 1 should have some Red and Green, no Blue
        assert!(out_frame[4] > 0); // R
        assert_eq!(out_frame[5], 255); // G
        assert_eq!(out_frame[6], 123); // B

        // Pixel 2 should be Blue
        assert_eq!(out_frame[8], 0);   // R
        assert_eq!(out_frame[9], 0);   // G
        assert_eq!(out_frame[10], 255); // B
    }

    #[test]
    fn ignores_invalid_frame_sizes() {
        let mut heatmap = VisualHeatmap::new();
        let short_frame = vec![0u8; 100];

        heatmap.track_frame(&short_frame);
        assert!(!heatmap.has_last_frame); // Should abort tracking
    }

    #[test]
    fn clears_state_correctly() {
        let mut heatmap = VisualHeatmap::new();
        let frame1 = vec![0u8; FRAME_RGBA_BYTES];
        let mut frame2 = vec![0u8; FRAME_RGBA_BYTES];

        frame2[0] = 255;
        heatmap.track_frame(&frame1);
        heatmap.track_frame(&frame2);

        assert_eq!(heatmap.max_count, 1);

        heatmap.clear();
        assert_eq!(heatmap.max_count, 0);
        assert!(!heatmap.has_last_frame);
        assert_eq!(heatmap.change_counts[0], 0);
    }
}
