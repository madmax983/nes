//! Experimental visualizer for drawing an audio oscilloscope on the framebuffer.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
/// A visualizer that draws an oscilloscope waveform representing audio samples directly onto the RGBA framebuffer.
///
/// `AudioOscilloscope` maps 16-bit PCM audio samples to the Y-axis of the framebuffer and draws a continuous
/// line across the X-axis. This allows visualizing audio output in real-time overlaid on the game screen.
pub struct AudioOscilloscope;

#[cfg(feature = "nova")]
impl AudioOscilloscope {
    /// Draws the audio waveform on the provided RGBA framebuffer.
    ///
    /// `samples` is a slice of 16-bit PCM audio data.
    /// `frame` must be a valid RGBA framebuffer slice of length `FRAME_WIDTH * FRAME_HEIGHT * 4`.
    pub fn draw_waveform(samples: &[i16], frame: &mut [u8], color: [u8; 4]) {
        if samples.is_empty() {
            return;
        }

        let num_samples = samples.len();
        let mut prev_y: Option<usize> = None;

        for x in 0..FRAME_WIDTH {
            let sample_idx = (x * num_samples) / FRAME_WIDTH;
            let sample = if sample_idx < num_samples {
                samples[sample_idx]
            } else {
                0
            };

            let normalized = (f32::from(sample) / 32768.0) * 0.4;
            let y_float = (FRAME_HEIGHT as f32 / 2.0) - (normalized * FRAME_HEIGHT as f32);
            let y = y_float.clamp(0.0, (FRAME_HEIGHT - 1) as f32) as usize;

            if let Some(py) = prev_y {
                let start_y = py.min(y);
                let end_y = py.max(y);
                for curr_y in start_y..=end_y {
                    Self::draw_pixel(frame, x, curr_y, color);
                }
            } else {
                Self::draw_pixel(frame, x, y, color);
            }
            prev_y = Some(y);
        }
    }

    fn draw_pixel(frame: &mut [u8], x: usize, y: usize, color: [u8; 4]) {
        if x < FRAME_WIDTH && y < FRAME_HEIGHT {
            let base_idx = (y * FRAME_WIDTH + x) * 4;
            if base_idx + 3 < frame.len() {
                frame[base_idx] = color[0];
                frame[base_idx + 1] = color[1];
                frame[base_idx + 2] = color[2];
                frame[base_idx + 3] = color[3];
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::constants::FRAME_RGBA_BYTES;

    #[test]
    fn test_oscilloscope_modifies_framebuffer() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        let samples = vec![32767, -32768, 0, 16384, -16384];

        let color = [0, 255, 0, 255]; // Green

        AudioOscilloscope::draw_waveform(&samples, &mut frame, color);

        let mut modified = false;
        for chunk in frame.chunks_exact(4) {
            if chunk[0] == 0 && chunk[1] == 255 && chunk[2] == 0 && chunk[3] == 255 {
                modified = true;
                break;
            }
        }
        assert!(modified, "Framebuffer was not modified by the oscilloscope");
    }

    #[test]
    fn test_oscilloscope_empty_samples() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        AudioOscilloscope::draw_waveform(&[], &mut frame, [255, 255, 255, 255]);
        assert!(frame.iter().all(|&b| b == 0));
    }
}
