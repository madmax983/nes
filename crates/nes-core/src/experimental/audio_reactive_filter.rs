//! Experimental audio-reactive visualizer.
//!
//! Analyzes the recent audio output amplitude and applies visual distortions
//! (like screen shake or chromatic aberration) to the framebuffer.
//! This creates a synesthetic experience where loud sound effects or music
//! physically impact the emulator's visual output.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Types of audio-reactive distortions.
pub enum DistortionEffect {
    /// Shifts the RGB channels horizontally based on audio amplitude.
    ChromaticAberration,
    /// Vertically shifts the entire frame based on audio amplitude (simulated screen shake).
    ScreenShake,
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone)]
/// An audio-reactive filter that modifies the framebuffer based on audio amplitude.
pub struct AudioReactiveFilter {
    effect: DistortionEffect,
    sensitivity: f32,
}

#[cfg(feature = "nova")]
impl AudioReactiveFilter {
    /// Creates a new audio-reactive filter.
    ///
    /// * `effect` - The visual distortion to apply.
    /// * `sensitivity` - A multiplier for the audio amplitude (e.g., `1.0` is default, `2.0` is twice as reactive).
    #[must_use]
    pub fn new(effect: DistortionEffect, sensitivity: f32) -> Self {
        Self {
            effect,
            sensitivity,
        }
    }

    /// Calculates the normalized amplitude (0.0 to 1.0) of a chunk of audio samples.
    fn calculate_amplitude(samples: &[i16]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let mut sum_squares = 0.0f32;
        for &sample in samples {
            // Normalize sample to -1.0 .. 1.0
            let normalized = f32::from(sample) / 32768.0;
            sum_squares += normalized * normalized;
        }

        let rms = (sum_squares / samples.len() as f32).sqrt();
        // Scale up RMS a bit so normal gameplay music has a noticeable effect
        (rms * 4.0).min(1.0)
    }

    /// Applies the visual distortion to the given RGBA framebuffer based on the provided audio samples.
    pub fn apply(&self, framebuffer: &mut [u8], audio_samples: &[i16]) {
        let amplitude = Self::calculate_amplitude(audio_samples) * self.sensitivity;
        if amplitude < 0.01 {
            return; // Too quiet to cause a distortion
        }

        let width = FRAME_WIDTH;
        let height = FRAME_HEIGHT;

        match self.effect {
            DistortionEffect::ChromaticAberration => {
                // Shift red channel left, blue channel right
                let shift_amount = (amplitude * 10.0) as usize; // Max 10 pixels shift per unit of amplitude
                if shift_amount == 0 {
                    return;
                }

                // We need a copy of the buffer to read from while we write
                let original = framebuffer.to_vec();

                for y in 0..height {
                    for x in 0..width {
                        let idx = (y * width + x) * 4;

                        // Red shift (left)
                        if x >= shift_amount {
                            let src_idx = (y * width + (x - shift_amount)) * 4;
                            framebuffer[idx] = original[src_idx]; // R
                        }

                        // Blue shift (right)
                        if x + shift_amount < width {
                            let src_idx = (y * width + (x + shift_amount)) * 4;
                            framebuffer[idx + 2] = original[src_idx + 2]; // B
                        }
                    }
                }
            }
            DistortionEffect::ScreenShake => {
                // Shift the entire image up/down
                let mut shift_amount = (amplitude * 15.0) as isize; // Max 15 pixels shake per unit of amplitude

                // Pseudo-random direction based on amplitude to make it "shake"
                if ((amplitude * 100.0) as u32).is_multiple_of(2) {
                    shift_amount = -shift_amount;
                }

                if shift_amount == 0 {
                    return;
                }

                let original = framebuffer.to_vec();
                // Clear framebuffer (fill with black)
                framebuffer.fill(0);

                for y in 0..height {
                    let new_y = y as isize + shift_amount;
                    if new_y >= 0 && new_y < height as isize {
                        let src_row_start = (y * width) * 4;
                        let src_row_end = src_row_start + width * 4;

                        let dst_row_start = (new_y as usize * width) * 4;
                        let dst_row_end = dst_row_start + width * 4;

                        framebuffer[dst_row_start..dst_row_end]
                            .copy_from_slice(&original[src_row_start..src_row_end]);
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_amplitude() {
        let quiet = vec![0; 100];
        assert_eq!(AudioReactiveFilter::calculate_amplitude(&quiet), 0.0);

        let mut loud = vec![0; 100];
        loud[0] = 32767;
        loud[1] = -32768;
        assert!(AudioReactiveFilter::calculate_amplitude(&loud) > 0.0);
    }

    #[test]
    fn test_chromatic_aberration() {
        let mut frame = vec![0; FRAME_WIDTH * FRAME_HEIGHT * 4];
        // Draw a white dot in the center
        let center_idx = (120 * FRAME_WIDTH + 128) * 4;
        frame[center_idx] = 255; // R
        frame[center_idx + 1] = 255; // G
        frame[center_idx + 2] = 255; // B
        frame[center_idx + 3] = 255; // A

        let filter = AudioReactiveFilter::new(DistortionEffect::ChromaticAberration, 10.0);

        // Very loud audio
        let loud_audio = vec![32767; 1024];
        filter.apply(&mut frame, &loud_audio);

        // The center dot's Red and Blue channels should have shifted out
        // meaning the center should no longer be pure white.
        assert!(frame[center_idx] != 255 || frame[center_idx + 2] != 255);
    }

    #[test]
    fn test_screen_shake() {
        let mut frame = vec![0; FRAME_WIDTH * FRAME_HEIGHT * 4];
        let center_idx = (120 * FRAME_WIDTH + 128) * 4;
        frame[center_idx] = 255;

        let filter = AudioReactiveFilter::new(DistortionEffect::ScreenShake, 10.0);
        let loud_audio = vec![32767; 1024];
        filter.apply(&mut frame, &loud_audio);

        // Center dot should have moved, so it shouldn't be at the center anymore
        assert_eq!(frame[center_idx], 0);
    }
}
