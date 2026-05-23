#![cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

pub struct AudioOscilloscope {
    color: (u8, u8, u8, u8),
}

impl Default for AudioOscilloscope {
    fn default() -> Self {
        Self::new((0, 255, 0, 255))
    }
}

impl AudioOscilloscope {
    #[must_use]
    pub fn new(color: (u8, u8, u8, u8)) -> Self {
        Self { color }
    }

    pub fn draw(&self, samples: &[i16], framebuffer: &mut [u8]) {
        if samples.is_empty() {
            return;
        }

        let num_points = FRAME_WIDTH.min(samples.len());
        let step = samples.len() as f32 / num_points as f32;
        let mid_y = FRAME_HEIGHT / 2;
        let amplitude_scale = (FRAME_HEIGHT as f32 / 2.0) / 32768.0;

        for x in 0..num_points {
            let sample_idx = (x as f32 * step) as usize;
            let sample = samples[sample_idx.min(samples.len() - 1)];

            let y_offset = (sample as f32 * amplitude_scale) as i32;
            let y = (mid_y as i32 - y_offset).clamp(0, (FRAME_HEIGHT - 1) as i32) as usize;

            let pixel_idx = (y * FRAME_WIDTH + x) * 4;
            if pixel_idx + 3 < framebuffer.len() {
                framebuffer[pixel_idx] = self.color.0;
                framebuffer[pixel_idx + 1] = self.color.1;
                framebuffer[pixel_idx + 2] = self.color.2;
                framebuffer[pixel_idx + 3] = self.color.3;
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    #[test]
    fn test_draw_modifies_framebuffer() {
        let osc = AudioOscilloscope::new((255, 0, 0, 255));
        let mut frame = vec![0; FRAME_WIDTH * FRAME_HEIGHT * 4];
        let samples = vec![32767, -32768, 0, 1000];
        osc.draw(&samples, &mut frame);
        let mut found_red = false;
        for pixel in frame.chunks_exact(4) {
            if pixel[0] == 255 && pixel[1] == 0 && pixel[2] == 0 && pixel[3] == 255 {
                found_red = true;
                break;
            }
        }
        assert!(found_red, "Framebuffer should be modified by draw()");
    }
}
