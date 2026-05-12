//! Experimental audio oscilloscope visualizer.
//!
//! This module provides an overlay that visualizes audio samples
//! as an oscilloscope-style waveform directly on the RGBA framebuffer.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
/// A post-processing visualizer that renders audio samples as a waveform.
///
/// `AudioOscilloscope` draws a horizontal waveform overlay on a raw RGBA
/// framebuffer. It maps audio amplitude to the Y-axis and interpolates
/// sample data across the X-axis (`FRAME_WIDTH`), connecting the dots
/// to form a continuous line.
pub struct AudioOscilloscope;

#[cfg(feature = "nova")]
impl AudioOscilloscope {
    /// Draws an oscilloscope waveform directly onto the provided RGBA framebuffer.
    ///
    /// * `frame` - Must be exactly `FRAME_WIDTH * FRAME_HEIGHT * 4` bytes.
    /// * `samples` - The 16-bit PCM audio samples to visualize. If empty, a flat line is drawn.
    /// * `color_rgba` - The `[R, G, B, A]` color used to draw the waveform line.
    /// * `y_center` - The Y-coordinate (0..`FRAME_HEIGHT`) where the baseline of the waveform will be drawn.
    /// * `height_amplitude` - The maximum height in pixels the waveform can reach above or below the baseline.
    pub fn draw_waveform(
        frame: &mut [u8],
        samples: &[i16],
        color_rgba: [u8; 4],
        y_center: usize,
        height_amplitude: usize,
    ) {
        if frame.len() != FRAME_WIDTH * FRAME_HEIGHT * 4 {
            return; // Safety bounds check
        }

        let y_center = y_center.clamp(0, FRAME_HEIGHT - 1);

        if samples.is_empty() {
            // Draw a flat baseline
            for x in 0..FRAME_WIDTH {
                Self::draw_pixel(frame, x, y_center, color_rgba);
            }
            return;
        }

        let max_sample_val = 32768.0; // i16 amplitude
        let mut prev_y: Option<usize> = None;

        for x in 0..FRAME_WIDTH {
            // Map X pixel to a sample index
            let sample_idx = (x * samples.len()) / FRAME_WIDTH;
            let sample_idx = sample_idx.min(samples.len() - 1);
            let sample = samples[sample_idx];

            // Normalize sample to [-1.0, 1.0]
            let normalized = (f64::from(sample) / max_sample_val).clamp(-1.0, 1.0);

            // Map to Y coordinate
            // Negative amplitude goes UP visually, positive goes DOWN
            let offset = (normalized * height_amplitude as f64).round() as isize;

            // Calculate raw target Y, allowing negative temporarily
            let target_y_raw = y_center as isize - offset;

            // Clamp to screen bounds
            let target_y = target_y_raw.clamp(0, (FRAME_HEIGHT - 1) as isize) as usize;

            if let Some(py) = prev_y {
                // Draw vertical connecting line between previous Y and current Y
                let y_start = py.min(target_y);
                let y_end = py.max(target_y);
                for y in y_start..=y_end {
                    Self::draw_pixel(frame, x, y, color_rgba);
                }
            } else {
                Self::draw_pixel(frame, x, target_y, color_rgba);
            }

            prev_y = Some(target_y);
        }
    }

    fn draw_pixel(frame: &mut [u8], x: usize, y: usize, color: [u8; 4]) {
        let idx = (y * FRAME_WIDTH + x) * 4;
        // Optimization: avoiding slice boundary checks inside loop, outer function verified len
        if idx + 3 < frame.len() {
            frame[idx] = color[0];
            frame[idx + 1] = color[1];
            frame[idx + 2] = color[2];
            frame[idx + 3] = color[3];
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

    #[test]
    fn flat_line_when_samples_empty() {
        let mut frame = vec![0_u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
        let color = [255, 0, 0, 255]; // Red

        AudioOscilloscope::draw_waveform(&mut frame, &[], color, 120, 50);

        // Check middle line is red
        for x in 0..FRAME_WIDTH {
            let idx = (120 * FRAME_WIDTH + x) * 4;
            assert_eq!(frame[idx], 255);
            assert_eq!(frame[idx + 1], 0);
            assert_eq!(frame[idx + 2], 0);
            assert_eq!(frame[idx + 3], 255);
        }

        // Check pixel above is empty
        let above_idx = (119 * FRAME_WIDTH + 10) * 4;
        assert_eq!(frame[above_idx], 0);
    }

    #[test]
    fn waveform_draws_correctly() {
        let mut frame = vec![0_u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
        let color = [0, 255, 0, 255]; // Green

        // Fake sine-ish wave: 0, Max, 0, Min
        let samples = vec![0, 32767, 0, -32768];

        AudioOscilloscope::draw_waveform(&mut frame, &samples, color, 120, 50);

        // At x = 0, sample index = 0, sample = 0, y = 120
        let idx0 = (120 * FRAME_WIDTH) * 4;
        assert_eq!(frame[idx0], 0);
        assert_eq!(frame[idx0 + 1], 255);

        // At x = FRAME_WIDTH/4 (index 64), sample index = 1, sample = 32767, y = 120 - 50 = 70
        let idx_peak = (70 * FRAME_WIDTH + 64) * 4;
        assert_eq!(frame[idx_peak], 0);
        assert_eq!(frame[idx_peak + 1], 255);
    }

    #[test]
    fn bounds_check_invalid_frame_size() {
        let mut tiny_frame = vec![0_u8; 10];
        let color = [255, 255, 255, 255];
        let samples = vec![0];
        // Should not panic
        AudioOscilloscope::draw_waveform(&mut tiny_frame, &samples, color, 0, 0);
        // Frame should remain untouched
        assert_eq!(tiny_frame, vec![0_u8; 10]);
    }
}
