//! Experimental Audio Oscilloscope visualizer.
//!
//! This module draws a waveform representation of the APU's audio output
//! directly onto the PPU framebuffer, creating a built-in oscilloscope overlay.
//! This is an additive feature designed for debug visualizers or music-player modes.

#[cfg(feature = "nova")]
/// A visualizer that renders an audio waveform overlay on a framebuffer.
pub struct AudioOscilloscope;

#[cfg(feature = "nova")]
impl AudioOscilloscope {
    /// Overlays a green oscilloscope waveform onto the framebuffer.
    ///
    /// The `framebuffer` should be an RGBA slice of length `width * height * 4`.
    /// The `audio_samples` should be 16-bit PCM samples from the APU.
    pub fn draw_overlay(
        framebuffer: &mut [u8],
        width: usize,
        height: usize,
        audio_samples: &[i16],
    ) {
        if audio_samples.is_empty() || framebuffer.len() < width * height * 4 {
            return;
        }

        let center_y = height / 2;
        let amplitude = height as f32 / 4.0; // +/- 25% of screen height

        // Find max absolute value to normalize the waveform
        let mut max_val = 1.0f32;
        for &s in audio_samples {
            let abs_s = s.abs() as f32;
            if abs_s > max_val {
                max_val = abs_s;
            }
        }

        let samples_per_pixel = audio_samples.len() as f32 / width as f32;

        for x in 0..width {
            let sample_idx = ((x as f32) * samples_per_pixel) as usize;
            let sample_idx = sample_idx.min(audio_samples.len() - 1);

            let normalized = audio_samples[sample_idx] as f32 / max_val;
            let y_offset = (normalized * amplitude) as isize;

            let y = (center_y as isize - y_offset).clamp(0, (height - 1) as isize) as usize;

            let pixel_idx = (y * width + x) * 4;
            if pixel_idx + 3 < framebuffer.len() {
                // Draw neon green pixel (R, G, B, A)
                framebuffer[pixel_idx] = 50;
                framebuffer[pixel_idx + 1] = 255;
                framebuffer[pixel_idx + 2] = 50;
                framebuffer[pixel_idx + 3] = 255;
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_draw_overlay_empty_samples() {
        let mut framebuffer = vec![0; 100 * 100 * 4];
        let original = framebuffer.clone();
        AudioOscilloscope::draw_overlay(&mut framebuffer, 100, 100, &[]);
        assert_eq!(
            framebuffer, original,
            "Framebuffer should not change if audio_samples is empty"
        );
    }

    #[test]
    fn test_draw_overlay_modifies_framebuffer() {
        let width = 100;
        let height = 100;
        let mut framebuffer = vec![0; width * height * 4];

        // Generate a simple sine wave for test
        let mut audio_samples = vec![0i16; 200];
        for (i, sample) in audio_samples.iter_mut().enumerate() {
            let t = i as f32 / 200.0;
            *sample = ((t * std::f32::consts::TAU).sin() * 10000.0) as i16;
        }

        AudioOscilloscope::draw_overlay(&mut framebuffer, width, height, &audio_samples);

        // Ensure at least one green pixel was drawn
        let mut has_green = false;
        for chunk in framebuffer.chunks_exact(4) {
            if chunk[0] == 50 && chunk[1] == 255 && chunk[2] == 50 && chunk[3] == 255 {
                has_green = true;
                break;
            }
        }
        assert!(
            has_green,
            "Expected oscilloscope to draw green pixels onto the framebuffer"
        );
    }

    #[test]
    fn test_draw_overlay_out_of_bounds_handling() {
        // Provide a framebuffer that's too small for the width/height
        let mut framebuffer = vec![0; 10];
        let audio_samples = vec![1000, 2000, -1000];

        // Should not panic
        AudioOscilloscope::draw_overlay(&mut framebuffer, 100, 100, &audio_samples);
    }
}
