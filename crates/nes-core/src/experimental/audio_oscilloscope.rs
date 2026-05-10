//! Experimental tool for visualizing audio output as an oscilloscope trace.

#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;

#[cfg(feature = "nova")]
pub struct AudioOscilloscope;

#[cfg(feature = "nova")]
impl AudioOscilloscope {
    /// Renders a slice of audio samples as a 256x256 BMP oscilloscope image.
    pub fn render_bmp(samples: &[i16]) -> Result<Vec<u8>, String> {
        let width = 256;
        let height = 256;
        let mut rgba = vec![0u8; width * height * 4];

        for x in 0..width {
            let idx = (128 * width + x) * 4;
            rgba[idx] = 0;
            rgba[idx + 1] = 50;
            rgba[idx + 2] = 0;
            rgba[idx + 3] = 255;
        }

        if samples.is_empty() {
            return encode_bmp(width, height, &rgba);
        }

        let step = (samples.len() as f32 / width as f32).max(1.0);
        let mut prev_y: Option<usize> = None;

        for x in 0..width {
            let sample_idx = ((x as f32) * step) as usize;
            if sample_idx >= samples.len() {
                break;
            }

            let sample = samples[sample_idx] as f32;
            let normalized = (sample / 16000.0).clamp(-1.0, 1.0);

            let y = ((1.0 - normalized) * 127.5) as usize;
            let y = y.clamp(0, height - 1);

            if let Some(py) = prev_y {
                let start_y = py.min(y);
                let end_y = py.max(y);
                for dy in start_y..=end_y {
                    let idx = (dy * width + x) * 4;
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
    fn fails_if_not_implemented() {
        // This test will initially fail if we don't have the BMP encode (to satisfy red phase)
        // But actually, we just test the method directly. Let's make sure it handles empty array.
        let bmp = AudioOscilloscope::render_bmp(&[]).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }

    #[test]
    fn renders_audio_samples() {
        let samples = vec![0, 16000, -16000, 0];
        let bmp = AudioOscilloscope::render_bmp(&samples).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }
}
