//! Experimental audio-reactive video glitcher.
//!
//! This module introduces the `AudioGlitcher`, which creates a synesthesia-like effect
//! by applying visual distortion (horizontal tearing and chromatic aberration) to the
//! framebuffer based on the amplitude of the emulator's recent audio output.
//! It effectively turns loud sounds into VHS-style tracking glitches.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
/// An experimental effect that distorts the screen based on sound volume.
pub struct AudioGlitcher;

#[cfg(feature = "nova")]
impl AudioGlitcher {
    /// Applies audio-reactive distortion to an RGBA framebuffer.
    ///
    /// `frame` must be exactly `FRAME_WIDTH * FRAME_HEIGHT * 4` bytes.
    /// `audio_samples` should be a recent chunk of audio (e.g., the last frame's output).
    /// `intensity` scales the effect (1.0 is default, 0.0 disables it).
    pub fn apply_glitch(frame: &mut [u8], audio_samples: &[i16], intensity: f32) {
        if intensity <= 0.0 || audio_samples.is_empty() {
            return;
        }

        // Verify framebuffer size to prevent panics
        if frame.len() != (FRAME_WIDTH * FRAME_HEIGHT * 4) {
            return;
        }

        // Calculate RMS (Root Mean Square) volume of the audio chunk
        let mut sum_squares = 0.0;
        for &sample in audio_samples {
            let normalized = (sample as f32) / 32768.0;
            sum_squares += normalized * normalized;
        }
        let rms = (sum_squares / (audio_samples.len() as f32)).sqrt();

        // If it's too quiet, skip the effect to save CPU
        if rms < 0.01 {
            return;
        }

        // Glitch magnitude based on volume and intensity
        let magnitude = (rms * intensity * 50.0) as usize;
        if magnitude == 0 {
            return;
        }

        let mut temp_row = vec![0u8; FRAME_WIDTH * 4];

        for y in 0..FRAME_HEIGHT {
            let y_usize = y;
            let width = FRAME_WIDTH;
            // Create a pseudo-random shift for this row based on Y coordinate and RMS
            // Using a simple sine wave + modulo for "randomness"
            let shift_val = ((y as f32 * 0.1) + (rms * 100.0)).sin();

            // Only glitch some lines to create "tearing" bands
            if shift_val > 0.5 {
                let shift = ((shift_val * magnitude as f32) as usize) % width;
                if shift == 0 {
                    continue;
                }

                let row_start = y_usize * width * 4;
                let row_end = row_start + width * 4;
                let row_slice = &mut frame[row_start..row_end];

                // Copy to temp
                temp_row.copy_from_slice(row_slice);

                // Shift pixels right (wrap around)
                let shift_bytes = shift * 4;
                let remaining_bytes = (width * 4) - shift_bytes;

                row_slice[shift_bytes..].copy_from_slice(&temp_row[..remaining_bytes]);
                row_slice[..shift_bytes].copy_from_slice(&temp_row[remaining_bytes..]);

                // Add a slight RGB channel separation (chromatic aberration) on glitched lines
                for x in 0..width {
                    if x > 2 {
                        let idx = x * 4;
                        let prev_idx = (x - 2) * 4;
                        // Shift red channel left
                        row_slice[idx] = temp_row[prev_idx];
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::constants::FRAME_RGBA_BYTES;

    #[test]
    fn audio_glitcher_shifts_pixels_on_loud_audio() {
        let mut frame = vec![0u8; FRAME_RGBA_BYTES];
        // Draw a vertical white line down the middle
        let width = FRAME_WIDTH;
        for y in 0..(FRAME_HEIGHT) {
            let idx = (y * width + (width / 2)) * 4;
            frame[idx] = 255;
            frame[idx + 1] = 255;
            frame[idx + 2] = 255;
            frame[idx + 3] = 255;
        }

        // Create loud audio chunk (max amplitude square wave)
        let mut audio = vec![0; 735]; // Typical frame audio chunk size
        for (i, sample) in audio.iter_mut().enumerate() {
            *sample = if i % 2 == 0 { 32767 } else { -32768 };
        }

        AudioGlitcher::apply_glitch(&mut frame, &audio, 2.0);

        // Verify that the line is no longer perfectly straight
        let mut straight = true;
        for y in 0..(FRAME_HEIGHT) {
            let idx = (y * width + (width / 2)) * 4;
            if frame[idx] == 0 {
                straight = false;
                break;
            }
        }
        assert!(
            !straight,
            "The vertical line should be distorted by the glitcher"
        );
    }

    #[test]
    fn audio_glitcher_does_nothing_on_silence() {
        let mut frame = vec![0u8; FRAME_RGBA_BYTES];
        frame[0] = 42;

        let audio = vec![0; 735]; // Silence

        AudioGlitcher::apply_glitch(&mut frame, &audio, 2.0);

        assert_eq!(frame[0], 42); // Unchanged
    }
}
