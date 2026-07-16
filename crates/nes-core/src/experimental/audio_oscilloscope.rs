//! Experimental tools for visualizing audio output as an oscilloscope waveform.
//!
//! This module provides the `AudioOscilloscope` utility, allowing developers
//! to generate visual representations of the audio waveform.

#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;

#[cfg(feature = "nova")]
/// A utility for rendering 16-bit PCM audio samples as a visual waveform.
pub struct AudioOscilloscope;

#[cfg(feature = "nova")]
impl AudioOscilloscope {
    /// Renders an array of 16-bit PCM samples to a BMP image of the given width and height.
    ///
    /// The waveform is drawn in bright green on a black background, resembling a classic
    /// oscilloscope. Lines are drawn between adjacent sample points to ensure a continuous
    /// waveform trace even if the sample rate is low compared to the image width.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_core::experimental::audio_oscilloscope::AudioOscilloscope;
    ///
    /// let samples = vec![0, 16384, 32767, 16384, 0, -16384, -32768, -16384];
    /// let bmp_bytes = AudioOscilloscope::render_bmp(&samples, 256, 128).unwrap();
    /// assert_eq!(&bmp_bytes[0..2], b"BM");
    /// ```
    pub fn render_bmp(samples: &[i16], width: usize, height: usize) -> Result<Vec<u8>, String> {
        let mut rgba = vec![0u8; width * height * 4];

        if samples.is_empty() || width == 0 || height == 0 {
            return encode_bmp(width, height, &rgba);
        }

        let mut prev_x = 0;
        let mut prev_y = Self::sample_to_y(samples[0], height);

        for (i, &sample) in samples.iter().enumerate() {
            let x = (i * width) / samples.len();
            let y = Self::sample_to_y(sample, height);

            Self::draw_line(&mut rgba, width, height, prev_x, prev_y, x, y);

            prev_x = x;
            prev_y = y;
        }

        encode_bmp(width, height, &rgba)
    }

    fn sample_to_y(sample: i16, height: usize) -> usize {
        let normalized = (f32::from(sample) + 32768.0) / 65535.0;
        let clamped = normalized.clamp(0.0, 1.0);
        let y = ((1.0 - clamped) * (height as f32 - 1.0)) as usize;
        y.clamp(0, height - 1)
    }

    fn draw_line(
        rgba: &mut [u8],
        width: usize,
        height: usize,
        x0: usize,
        y0: usize,
        x1: usize,
        y1: usize,
    ) {
        let dx = (x1 as isize - x0 as isize).abs();
        let dy = -(y1 as isize - y0 as isize).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut cx = x0 as isize;
        let mut cy = y0 as isize;

        loop {
            if cx >= 0 && cx < width as isize && cy >= 0 && cy < height as isize {
                let idx = (cy as usize * width + cx as usize) * 4;
                rgba[idx] = 0; // R
                rgba[idx + 1] = 255; // G
                rgba[idx + 2] = 0; // B
                rgba[idx + 3] = 255; // A
            }

            if cx == x1 as isize && cy == y1 as isize {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                cx += sx;
            }
            if e2 <= dx {
                err += dx;
                cy += sy;
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_render_empty_samples() {
        let samples: Vec<i16> = vec![];
        let bmp = AudioOscilloscope::render_bmp(&samples, 100, 100).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }

    #[test]
    fn test_render_waveform() {
        let samples = vec![0, 10000, 20000, 32767, -10000, -32768];
        let bmp = AudioOscilloscope::render_bmp(&samples, 256, 128).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }
}
