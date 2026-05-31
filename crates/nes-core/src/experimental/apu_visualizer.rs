#![cfg(feature = "nova")]

use crate::bmp::encode_bmp;

/// Experimental visualizer for generating oscilloscope-style waveforms from APU audio samples.
pub struct ApuVisualizer;

impl ApuVisualizer {
    /// Renders an oscilloscope-style waveform of the provided audio samples.
    ///
    /// The resulting image is an RGBA BMP of dimensions `width` by `height`.
    pub fn render_waveform_bmp(
        samples: &[i16],
        width: usize,
        height: usize,
        bg_color: (u8, u8, u8),
        fg_color: (u8, u8, u8),
    ) -> Result<Vec<u8>, String> {
        if width == 0 || height == 0 {
            return Err("Width and height must be greater than zero".to_string());
        }

        let mut rgba = vec![0u8; width * height * 4];

        // Fill background
        for i in 0..(width * height) {
            rgba[i * 4] = bg_color.0;
            rgba[i * 4 + 1] = bg_color.1;
            rgba[i * 4 + 2] = bg_color.2;
            rgba[i * 4 + 3] = 255;
        }

        if samples.is_empty() {
            return encode_bmp(width, height, &rgba);
        }

        let half_height = height as f32 / 2.0;

        let mut draw_line = |mut x0: isize, mut y0: isize, x1: isize, y1: isize| {
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;

            loop {
                if x0 >= 0 && x0 < width as isize && y0 >= 0 && y0 < height as isize {
                    let pixel_idx = (y0 as usize * width + x0 as usize) * 4;
                    rgba[pixel_idx] = fg_color.0;
                    rgba[pixel_idx + 1] = fg_color.1;
                    rgba[pixel_idx + 2] = fg_color.2;
                    rgba[pixel_idx + 3] = 255;
                }

                if x0 == x1 && y0 == y1 {
                    break;
                }

                let e2 = 2 * err;
                if e2 >= dy {
                    err += dy;
                    x0 += sx;
                }
                if e2 <= dx {
                    err += dx;
                    y0 += sy;
                }
            }
        };

        let mut prev_x: isize = -1;
        let mut prev_y: isize = -1;

        for (i, &sample) in samples.iter().enumerate() {
            let x = ((i as f32 / samples.len() as f32) * width as f32) as isize;

            // Normalize sample from i16 range to -1.0..1.0
            let normalized_sample = sample as f32 / 32768.0;

            // Invert Y axis so positive is up
            let y = (half_height - (normalized_sample * half_height)) as isize;

            if prev_x != -1 && prev_y != -1 {
                draw_line(prev_x, prev_y, x, y);
            }
            prev_x = x;
            prev_y = y;
        }

        encode_bmp(width, height, &rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn render_waveform_returns_bmp_with_correct_header() {
        let samples = vec![0i16; 100];
        let bmp = ApuVisualizer::render_waveform_bmp(&samples, 100, 50, (0, 0, 0), (255, 255, 255))
            .unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }
}
