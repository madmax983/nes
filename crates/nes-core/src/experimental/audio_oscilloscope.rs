//! Experimental visualizer that renders an audio oscilloscope over the framebuffer.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
/// An experimental visualizer that draws an oscilloscope waveform representing the audio samples directly onto the RGBA framebuffer.
pub struct AudioOscilloscope;

#[cfg(feature = "nova")]
impl AudioOscilloscope {
    /// Draws the provided 16-bit PCM audio samples as a waveform on the framebuffer.
    pub fn draw_waveform(frame: &mut [u8], samples: &[i16], color: [u8; 4]) {
        if samples.is_empty() {
            return;
        }

        let width = FRAME_WIDTH;
        let baseline = FRAME_HEIGHT - 32;
        let amplitude_scale = 30.0 / 32768.0;

        let step = samples.len() as f32 / width as f32;

        let mut prev_x = 0;
        let mut prev_y = baseline;

        for x in 0..width {
            let sample_idx = (x as f32 * step) as usize;
            if sample_idx < samples.len() {
                let sample = samples[sample_idx];
                let offset = (sample as f32 * amplitude_scale) as isize;
                let y = (baseline as isize - offset).clamp(0, (FRAME_HEIGHT - 1) as isize) as usize;

                if x == 0 {
                    prev_x = x;
                    prev_y = y;
                }

                Self::draw_line(frame, prev_x, prev_y, x, y, color);

                prev_x = x;
                prev_y = y;
            }
        }
    }

    fn draw_line(frame: &mut [u8], x0: usize, y0: usize, x1: usize, y1: usize, color: [u8; 4]) {
        let mut x = x0 as isize;
        let mut y = y0 as isize;
        let end_x = x1 as isize;
        let end_y = y1 as isize;

        let dx = (end_x - x).abs();
        let sx = if x < end_x { 1 } else { -1 };
        let dy = -(end_y - y).abs();
        let sy = if y < end_y { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            Self::draw_pixel(frame, x as usize, y as usize, color);
            if x == end_x && y == end_y {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    fn draw_pixel(frame: &mut [u8], x: usize, y: usize, color: [u8; 4]) {
        let idx = (y * FRAME_WIDTH + x) * 4;
        if idx + 3 < frame.len() {
            frame[idx] = color[0];
            frame[idx + 1] = color[1];
            frame[idx + 2] = color[2];
            frame[idx + 3] = color[3];
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

    #[test]
    fn test_draw_waveform_modifies_framebuffer() {
        let mut frame = vec![0u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
        let mut samples = vec![0i16; 735];

        // Create a simple sine wave
        for (i, sample) in samples.iter_mut().enumerate().take(735) {
            let t = i as f32 / 735.0;
            *sample = (f32::sin(t * std::f32::consts::PI * 4.0) * 32767.0) as i16;
        }

        // Base condition before draw
        let baseline = FRAME_HEIGHT - 32;
        let baseline_idx = (baseline * FRAME_WIDTH + FRAME_WIDTH / 2) * 4;
        assert_eq!(frame[baseline_idx + 1], 0);

        AudioOscilloscope::draw_waveform(&mut frame, &samples, [0, 255, 0, 255]);

        // Count drawn pixels to ensure something happened
        let mut drawn_pixels = 0;
        for chunk in frame.chunks_exact(4) {
            if chunk == [0, 255, 0, 255] {
                drawn_pixels += 1;
            }
        }

        assert!(drawn_pixels > 100); // Should be roughly FRAME_WIDTH pixels
    }

    #[test]
    fn test_draw_waveform_empty_samples() {
        let mut frame = vec![0u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
        let samples = vec![];
        AudioOscilloscope::draw_waveform(&mut frame, &samples, [0, 255, 0, 255]);

        for byte in frame.iter() {
            assert_eq!(*byte, 0);
        }
    }
}
