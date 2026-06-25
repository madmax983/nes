//! Experimental tools for visualizing APU audio output as a waveform.
//!
//! This module provides the `AudioVisualizer`, which takes a buffer of PCM
//! audio samples and generates an oscilloscope-style waveform image. This
//! is useful for debugging sound channels, verifying pulse duty cycles,
//! or just creating cool visuals for a music player.

#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;

#[cfg(feature = "nova")]
/// A stateless visualizer that renders PCM audio samples into a waveform image.
pub struct AudioVisualizer;

#[cfg(feature = "nova")]
impl AudioVisualizer {
    /// Renders a buffer of 16-bit PCM audio samples into a BMP image.
    ///
    /// The resulting image is `width` x `height` pixels, with a black background
    /// and a bright green waveform trace.
    ///
    /// # Arguments
    ///
    /// * `samples` - A slice of interleaved 16-bit PCM audio samples.
    /// * `width` - The width of the output image in pixels.
    /// * `height` - The height of the output image in pixels.
    ///
    /// # Examples
    ///
    /// ```
    /// use nes_core::experimental::audio_visualizer::AudioVisualizer;
    ///
    /// // Generate a simple sawtooth wave
    /// let samples: Vec<i16> = (0..800).map(|i| ((i % 100) * 327 - 16384) as i16).collect();
    ///
    /// // Render to a 400x200 BMP
    /// let bmp_bytes = AudioVisualizer::render_waveform_bmp(&samples, 400, 200).unwrap();
    /// assert_eq!(&bmp_bytes[0..2], b"BM");
    /// ```
    pub fn render_waveform_bmp(
        samples: &[i16],
        width: usize,
        height: usize,
    ) -> Result<Vec<u8>, String> {
        if width == 0 || height == 0 || samples.is_empty() {
            return encode_bmp(width, height, &[]); // Will error appropriately
        }

        let mut rgba = vec![0u8; width * height * 4];

        // Helper to set a pixel (x, y) to green
        let set_pixel = |rgba: &mut [u8], x: usize, y: usize| {
            if x < width && y < height {
                let idx = (y * width + x) * 4;
                rgba[idx] = 0;
                rgba[idx + 1] = 255;
                rgba[idx + 2] = 0;
                rgba[idx + 3] = 255;
            }
        };

        // Draw lines between samples
        let center_y = height as f32 / 2.0;
        let scale_y = height as f32 / 65536.0; // i16 is -32768 to 32767

        for x in 0..width {
            let sample_idx = (x * samples.len()) / width;
            let sample = samples[sample_idx] as f32;
            let y = (center_y - (sample * scale_y)).clamp(0.0, (height - 1) as f32) as usize;

            let next_sample_idx = ((x + 1) * samples.len()) / width;
            let next_sample = if next_sample_idx < samples.len() {
                samples[next_sample_idx] as f32
            } else {
                sample
            };
            let next_y =
                (center_y - (next_sample * scale_y)).clamp(0.0, (height - 1) as f32) as usize;

            let min_y = y.min(next_y);
            let max_y = y.max(next_y);

            for draw_y in min_y..=max_y {
                set_pixel(&mut rgba, x, draw_y);
            }
        }

        encode_bmp(width, height, &rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn can_render_waveform() {
        let samples: Vec<i16> = (0..800).map(|i| ((i % 100) * 327 - 16384) as i16).collect();
        let bmp_data = AudioVisualizer::render_waveform_bmp(&samples, 400, 200).unwrap();
        assert_eq!(&bmp_data[0..2], b"BM");
        assert!(bmp_data.len() > 54);
    }

    #[test]
    fn trace_is_not_empty() {
        let samples = vec![16384, 0, -16384];
        let bmp_data = AudioVisualizer::render_waveform_bmp(&samples, 10, 10).unwrap();
        // Since we know the BMP format stores pixels starting at offset 54
        // Let's check if there's any non-zero byte in the pixel data.
        let pixel_data = &bmp_data[54..];
        let has_lit_pixel = pixel_data.iter().any(|&b| b > 0);
        assert!(
            has_lit_pixel,
            "The waveform trace should be visible (not solid black)"
        );
    }
}
