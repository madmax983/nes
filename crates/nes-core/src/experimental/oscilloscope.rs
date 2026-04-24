//! Experimental visualizer to render audio samples as an oscilloscope waveform.
//!
//! This module provides a way to plot raw audio sample values (from the APU)
//! onto a raw RGBA framebuffer, bridging the audio and video outputs.

#[cfg(feature = "nova")]
/// Configuration and state for the audio oscilloscope visualizer.
pub struct Oscilloscope {
    line_color: [u8; 4],
    bg_color: [u8; 4],
}

#[cfg(feature = "nova")]
impl Oscilloscope {
    /// Creates a new oscilloscope visualizer with specified colors.
    #[must_use]
    pub fn new(line_color: [u8; 4], bg_color: [u8; 4]) -> Self {
        Self {
            line_color,
            bg_color,
        }
    }

    /// Renders the given audio samples onto the RGBA framebuffer as a waveform.
    ///
    /// The `framebuffer` must be `width * height * 4` bytes.
    /// The samples will be drawn connected by lines (Bresenham's).
    pub fn render(&self, framebuffer: &mut [u8], width: usize, height: usize, samples: &[i16]) {
        // Clear background
        for chunk in framebuffer.chunks_exact_mut(4) {
            chunk.copy_from_slice(&self.bg_color);
        }

        if samples.is_empty() || width == 0 || height == 0 {
            return;
        }

        // Draw waveform
        // Map samples into coordinates: x goes from 0 to width, y mapped from i16 range to 0..height
        let sample_to_y = |sample: i16| -> isize {
            // map i16::MIN..i16::MAX to height-1..0
            // sample + 32768 goes from 0..65535
            let normalized = (i32::from(sample) + 32768) as f32 / 65535.0;
            // Invert y so positive values go up
            let y = (1.0 - normalized) * ((height.saturating_sub(1)) as f32);
            y.round() as isize
        };

        // We map the length of `samples` to `width`.
        // If we have fewer samples than width, we interpolate.
        // If we have more samples, we step through them.

        let mut prev_x: isize = 0;
        let mut prev_y: isize = sample_to_y(samples[0]);

        // Draw a line using Bresenham's from (x0, y0) to (x1, y1)
        let mut draw_line = |mut x0: isize, mut y0: isize, x1: isize, y1: isize| {
            let dx = (x1 - x0).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let dy = -(y1 - y0).abs();
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx + dy;

            loop {
                // Plot pixel
                if x0 >= 0 && x0 < (width as isize) && y0 >= 0 && y0 < (height as isize) {
                    let idx = ((y0 as usize) * width + (x0 as usize)) * 4;
                    if idx + 3 < framebuffer.len() {
                        framebuffer[idx..idx + 4].copy_from_slice(&self.line_color);
                    }
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

        for x in 1..width {
            // Find corresponding sample index
            let sample_idx = (x * samples.len()) / width;
            // Handle edge case where division gets us exactly samples.len()
            let sample_idx = sample_idx.min(samples.len().saturating_sub(1));

            let y = sample_to_y(samples[sample_idx]);

            draw_line(prev_x, prev_y, x as isize, y);

            prev_x = x as isize;
            prev_y = y;
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_oscilloscope_renders_waveform() {
        let width = 100;
        let height = 100;
        let mut framebuffer = vec![0; width * height * 4];

        let mut samples = vec![0; width];
        // Create a simple step pattern
        for (i, sample) in samples.iter_mut().enumerate() {
            if i < 50 {
                *sample = i16::MAX / 2; // Upper half
            } else {
                *sample = i16::MIN / 2; // Lower half
            }
        }

        let bg_color = [0, 0, 0, 255]; // Black
        let line_color = [0, 255, 0, 255]; // Green

        let osc = Oscilloscope::new(line_color, bg_color);
        osc.render(&mut framebuffer, width, height, &samples);

        // Check background is cleared to black at 0,0
        assert_eq!(&framebuffer[0..4], &bg_color);

        // Check that some green pixels exist (the waveform was drawn)
        let has_green = framebuffer.chunks_exact(4).any(|pixel| pixel == line_color);
        assert!(has_green, "Oscilloscope failed to render the waveform line");
    }
}
