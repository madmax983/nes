//! Experimental audio oscilloscope visualizer.
//!
//! This module provides the `AudioOscilloscope`, which visualizes raw NES audio
//! samples as a waveform drawn onto a framebuffer. This is useful for analyzing
//! APU output characteristics visually.

#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;

#[cfg(feature = "nova")]
/// A visualizer that draws an oscilloscope waveform from audio samples.
pub struct AudioOscilloscope;

#[cfg(feature = "nova")]
impl AudioOscilloscope {
    /// Draws the waveform representing the provided audio samples as a BMP image.
    ///
    /// The samples are expected to be 16-bit PCM values.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::audio_oscilloscope::AudioOscilloscope;
    /// let samples = vec![0, 1000, 2000, 0, -1000, -2000];
    /// let bmp = AudioOscilloscope::draw_waveform_bmp(&samples, 128, 64).unwrap();
    /// assert_eq!(&bmp[0..2], b"BM");
    /// ```
    pub fn draw_waveform_bmp(
        samples: &[i16],
        width: usize,
        height: usize,
    ) -> Result<Vec<u8>, String> {
        if height == 0 {
            return Err("Height must be greater than 0".to_string());
        }
        let mut rgba = vec![0u8; width * height * 4];

        for i in 0..(width * height) {
            rgba[i * 4 + 3] = 255;
        }

        let center_y = height / 2;
        for x in 0..width {
            let idx = (center_y * width + x) * 4;
            rgba[idx] = 50;
            rgba[idx + 1] = 50;
            rgba[idx + 2] = 50;
            rgba[idx + 3] = 255;
        }

        if samples.is_empty() {
            return encode_bmp(width, height, &rgba);
        }

        let samples_per_pixel = samples.len() as f32 / width as f32;
        let mut prev_y: Option<usize> = None;

        for x in 0..width {
            let start_idx = (x as f32 * samples_per_pixel) as usize;
            let mut end_idx = ((x + 1) as f32 * samples_per_pixel) as usize;
            if end_idx > samples.len() {
                end_idx = samples.len();
            }
            if start_idx >= samples.len() {
                break;
            }

            let mut sum = 0.0;
            for sample in samples.iter().take(end_idx).skip(start_idx) {
                sum += *sample as f32;
            }
            let avg = if end_idx > start_idx {
                sum / (end_idx - start_idx) as f32
            } else {
                samples[start_idx] as f32
            };

            let normalized = (avg + 32768.0) / 65535.0;
            let y = ((1.0 - normalized) * (height as f32 - 1.0)) as usize;
            let y = y.clamp(0, height - 1);

            if let Some(py) = prev_y {
                let y_start = py.min(y);
                let y_end = py.max(y);
                for cy in y_start..=y_end {
                    let idx = (cy * width + x) * 4;
                    rgba[idx] = 0;
                    rgba[idx + 1] = 255;
                    rgba[idx + 2] = 0;
                    rgba[idx + 3] = 255;
                }
            } else {
                let idx = (y * width + x) * 4;
                rgba[idx] = 0;
                rgba[idx + 1] = 255;
                rgba[idx + 2] = 0;
                rgba[idx + 3] = 255;
            }

            prev_y = Some(y);
        }

        encode_bmp(width, height, &rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_draw_waveform_bmp_creates_valid_bmp() {
        let samples = vec![0, 1000, 2000, 0, -1000, -2000];
        let bmp_data = AudioOscilloscope::draw_waveform_bmp(&samples, 128, 64).unwrap();
        assert_eq!(&bmp_data[0..2], b"BM");

        // Assert some pixel data instead of just the length
        let pixel_data_size = u32::from_le_bytes(bmp_data[34..38].try_into().unwrap());
        assert_eq!(pixel_data_size, 24576);
    }

    #[test]
    fn test_draw_waveform_draws_samples() {
        let samples = vec![32767, -32768, 0];
        let bmp_data = AudioOscilloscope::draw_waveform_bmp(&samples, 10, 10).unwrap();

        // Assert memory layout / specific pixel values instead of just checking the size
        assert_eq!(&bmp_data[0..2], b"BM");
        assert_eq!(bmp_data.len(), 374); // 54 header + 320 bytes pixel data

        // For a high sample, the y value should be low (e.g. 0), corresponding to the top of the image
        // Since BMP stores bottom-up, this means pixel row 9.
        // Row 9 starts at index: 54 + 9 * 32 (where 32 is the stride: 10 pixels * 3 bytes + 2 bytes padding)
        let top_left_idx = 54 + 9 * 32;
        // Depending on exact drawing logic, we expect a green pixel (BGR: 0, 255, 0)
        assert_eq!(&bmp_data[top_left_idx..top_left_idx + 3], &[0, 255, 0]);
    }
}
