use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

pub struct OscilloscopeVisualizer;

impl OscilloscopeVisualizer {
    pub fn draw_waveform(samples: &[i16], frame: &mut [u8]) {
        if samples.is_empty() {
            return;
        }
        let step = samples.len() as f32 / FRAME_WIDTH as f32;
        for x in 0..FRAME_WIDTH {
            let sample_idx = (x as f32 * step) as usize;
            let sample = samples.get(sample_idx).copied().unwrap_or(0);

            // Normalize i16 sample to Y coordinate (0 to FRAME_HEIGHT)
            let normalized = (sample as f32 / 32768.0) * 0.5 + 0.5;
            let y = (normalized * (FRAME_HEIGHT as f32 - 1.0)) as usize;
            let y = y.clamp(0, FRAME_HEIGHT - 1);

            let pixel_idx = (y * FRAME_WIDTH + x) * 4;
            if pixel_idx + 3 < frame.len() {
                frame[pixel_idx] = 0; // R
                frame[pixel_idx + 1] = 255; // G (Neon green)
                frame[pixel_idx + 2] = 0; // B
                frame[pixel_idx + 3] = 255; // A
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::constants::FRAME_RGBA_BYTES;

    #[test]
    fn test_draw_waveform_modifies_frame() {
        let samples = vec![1000; 256];
        let mut frame = vec![0; FRAME_RGBA_BYTES];
        OscilloscopeVisualizer::draw_waveform(&samples, &mut frame);
        assert!(frame.iter().any(|&p| p > 0), "Frame should be modified");
    }
}
