//! Experimental audio oscilloscope visualizer.
//!
//! This module introduces the `AudioOscilloscope`, which draws a waveform of the
//! recent audio samples directly onto the framebuffer, creating a built-in
//! music visualizer aesthetic.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
/// An experimental effect that draws an audio waveform on the screen.
pub struct AudioOscilloscope;

#[cfg(feature = "nova")]
impl AudioOscilloscope {
    /// Draws an audio waveform on an RGBA framebuffer.
    ///
    /// `frame` must be exactly `FRAME_WIDTH * FRAME_HEIGHT * 4` bytes.
    /// `audio_samples` should be a recent chunk of audio (e.g., the last frame's output).
    /// `color` is the RGBA color of the waveform line.
    pub fn draw_waveform(frame: &mut [u8], audio_samples: &[i16], color: [u8; 4]) {
        if audio_samples.is_empty() {
            return;
        }

        // Verify framebuffer size to prevent panics
        if frame.len() != FRAME_WIDTH * FRAME_HEIGHT * 4 {
            return;
        }

        let width = FRAME_WIDTH;
        let height = FRAME_HEIGHT;
        let mid_y = height / 2;

        // We want to map the audio samples across the width of the screen.
        // We'll downsample or upsample the audio to fit the screen width.
        let samples_per_pixel = (audio_samples.len() as f32 / width as f32).max(1.0);

        let mut prev_x = 0;
        let mut prev_y = mid_y;

        for x in 0..width {
            let sample_idx = (x as f32 * samples_per_pixel) as usize;
            if sample_idx >= audio_samples.len() {
                break;
            }

            let sample = audio_samples[sample_idx];

            // Map sample (-32768 to 32767) to Y coordinate (height - 1 to 0)
            // A sample of 0 maps to mid_y.
            let normalized = (sample as f32) / 32768.0;
            // Scale so max amplitude takes up roughly half the screen height (quarter above, quarter below)
            let y_offset = (normalized * (height as f32 / 4.0)) as isize;

            let y = (mid_y as isize - y_offset).clamp(0, (height - 1) as isize) as usize;

            // Draw a line from (prev_x, prev_y) to (x, y)
            Self::draw_line(frame, prev_x, prev_y, x, y, color);

            prev_x = x;
            prev_y = y;
        }
    }

    /// Bresenham's line algorithm to draw a line on the framebuffer
    fn draw_line(
        frame: &mut [u8],
        mut x0: usize,
        mut y0: usize,
        x1: usize,
        y1: usize,
        color: [u8; 4],
    ) {
        let dx = (x1 as isize - x0 as isize).abs();
        let dy = -(y1 as isize - y0 as isize).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let width = FRAME_WIDTH;
        let height = FRAME_HEIGHT;

        loop {
            if x0 < width && y0 < height {
                let idx = (y0 * width + x0) * 4;
                if idx + 3 < frame.len() {
                    frame[idx] = color[0];
                    frame[idx + 1] = color[1];
                    frame[idx + 2] = color[2];
                    frame[idx + 3] = color[3];
                }
            }

            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 = (x0 as isize + sx) as usize;
            }
            if e2 <= dx {
                err += dx;
                y0 = (y0 as isize + sy) as usize;
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn audio_oscilloscope_draws_line_for_non_zero_audio() {
        let mut frame = vec![0u8; FRAME_WIDTH * FRAME_HEIGHT * 4];

        // Create audio chunk (square wave)
        let mut audio = vec![0; 735];
        for (i, sample) in audio.iter_mut().enumerate() {
            *sample = if i % 100 < 50 { 16384 } else { -16384 };
        }

        let color = [255, 0, 0, 255]; // Red
        AudioOscilloscope::draw_waveform(&mut frame, &audio, color);

        // Verify that some red pixels were drawn
        let mut drawn = false;
        for i in (0..frame.len()).step_by(4) {
            if frame[i] == 255 && frame[i + 1] == 0 && frame[i + 2] == 0 {
                drawn = true;
                break;
            }
        }
        assert!(
            drawn,
            "The oscilloscope should have drawn red pixels on the frame"
        );
    }

    #[test]
    fn audio_oscilloscope_does_nothing_on_empty_audio() {
        let mut frame = vec![0u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
        let original_frame = frame.clone();

        let audio: Vec<i16> = vec![];

        AudioOscilloscope::draw_waveform(&mut frame, &audio, [255, 255, 255, 255]);

        assert_eq!(frame, original_frame);
    }
}
