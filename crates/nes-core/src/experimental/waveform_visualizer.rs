#![cfg(feature = "nova")]

//! Experimental audio waveform visualizer.
//!
//! This module renders raw PCM audio samples into an oscilloscope-style BMP image.

use crate::bmp::encode_bmp;

/// A utility for rendering an oscilloscope-style view of a slice of audio samples.
pub struct WaveformVisualizer;

impl WaveformVisualizer {
    /// Renders a waveform of the given 16-bit PCM samples to a BMP image.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::waveform_visualizer::WaveformVisualizer;
    /// let samples = vec![0, 16384, 32767, 16384, 0, -16384, -32768, -16384];
    /// let bmp = WaveformVisualizer::extract_waveform_bmp(&samples, 128, 64).unwrap();
    /// assert_eq!(&bmp[0..2], b"BM");
    /// ```
    pub fn extract_waveform_bmp(
        samples: &[i16],
        width: usize,
        height: usize,
    ) -> Result<Vec<u8>, String> {
        let mut rgba = vec![0u8; width * height * 4];

        // Fill background with dark gray
        for chunk in rgba.chunks_exact_mut(4) {
            chunk[0] = 20;
            chunk[1] = 20;
            chunk[2] = 20;
            chunk[3] = 255;
        }

        if samples.is_empty() || width == 0 || height == 0 {
            return encode_bmp(width, height, &rgba);
        }

        let samples_per_pixel = (samples.len() as f32 / width as f32).max(1.0);
        let center_y = height as i32 / 2;
        let max_amplitude = 32768.0;

        let mut prev_y = center_y;

        for x in 0..width {
            let sample_idx = ((x as f32) * samples_per_pixel) as usize;
            let sample = if sample_idx < samples.len() {
                samples[sample_idx]
            } else {
                *samples.last().unwrap()
            };

            let normalized = f32::from(sample) / max_amplitude; // -1.0 to 1.0

            // In `encode_bmp`, y=0 is output last, so it's visually at the top of the image.
            // Positive amplitude should go UP, meaning a smaller Y index.
            let mut y = center_y - (normalized * (height as f32 / 2.0)) as i32;
            y = y.clamp(0, height as i32 - 1);

            // Draw line from prev_y to y
            let (y0, y1) = if prev_y < y { (prev_y, y) } else { (y, prev_y) };
            for ly in y0..=y1 {
                let pixel_idx = (ly as usize * width + x) * 4;
                rgba[pixel_idx] = 0; // R
                rgba[pixel_idx + 1] = 255; // G
                rgba[pixel_idx + 2] = 0; // B
                rgba[pixel_idx + 3] = 255; // A
            }
            prev_y = y;
        }

        encode_bmp(width, height, &rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn extract_waveform_creates_bmp() {
        let samples = vec![0, 1000, 2000, -1000, -2000];
        let bmp = WaveformVisualizer::extract_waveform_bmp(&samples, 100, 50).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }

    #[test]
    fn extract_waveform_handles_empty_samples() {
        let samples = vec![];
        let bmp = WaveformVisualizer::extract_waveform_bmp(&samples, 10, 10).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }
}
