//! Experimental visualizer that draws an audio oscilloscope on the framebuffer.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
/// An overlay that draws an oscilloscope representation of audio samples onto the PPU framebuffer.
pub struct OscilloscopeOverlay;

#[cfg(feature = "nova")]
impl OscilloscopeOverlay {
    /// Draws an oscilloscope wave on the bottom portion of the given RGBA framebuffer.
    ///
    /// `frame` must be a valid RGBA buffer of size `FRAME_WIDTH * FRAME_HEIGHT * 4`.
    /// `samples` contains the `i16` audio samples to visualize. The samples will be
    /// scaled and drawn as a neon green wave.
    pub fn draw(frame: &mut [u8], samples: &[i16]) {
        if frame.len() != FRAME_WIDTH * FRAME_HEIGHT * 4 {
            return;
        }

        if samples.is_empty() {
            return;
        }

        // We'll draw on the bottom 64 rows of the screen
        let bottom_margin = 8;
        let height = 64;
        let base_y = FRAME_HEIGHT - bottom_margin - (height / 2);

        // Max magnitude of i16 is 32768
        let scale = (height as f32) / 65536.0;

        for x in 0..FRAME_WIDTH {
            let sample_idx = (x * samples.len()) / FRAME_WIDTH;
            if sample_idx >= samples.len() {
                break;
            }
            let sample = samples[sample_idx];

            let y_offset = (sample as f32 * scale) as isize;
            let mut y = (base_y as isize) - y_offset;

            // Clamp y to screen bounds
            if y < 0 {
                y = 0;
            }
            if y >= FRAME_HEIGHT as isize {
                y = (FRAME_HEIGHT - 1) as isize;
            }

            let y = y as usize;

            let pixel_idx = (y * FRAME_WIDTH + x) * 4;
            if pixel_idx + 3 < frame.len() {
                // Neon Green
                frame[pixel_idx] = 0;
                frame[pixel_idx + 1] = 255;
                frame[pixel_idx + 2] = 0;
                frame[pixel_idx + 3] = 255;
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::constants::{FRAME_RGBA_BYTES, FRAME_WIDTH};

    #[test]
    fn oscilloscope_draws_wave_on_framebuffer() {
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];
        // Flatline wave
        let samples = vec![0; FRAME_WIDTH as usize];

        OscilloscopeOverlay::draw(&mut frame, &samples);

        // Verify some pixels were turned green
        let mut has_green = false;
        for pixel in frame.chunks_exact(4) {
            if pixel[1] == 255 && pixel[0] == 0 && pixel[2] == 0 {
                has_green = true;
                break;
            }
        }
        assert!(
            has_green,
            "Oscilloscope should have drawn green pixels on the frame"
        );
    }

    #[test]
    fn oscilloscope_ignores_invalid_framebuffer() {
        let mut frame = vec![0_u8; 3]; // Invalid length
        let samples = vec![0; 10];
        OscilloscopeOverlay::draw(&mut frame, &samples); // Should not panic
    }
}
