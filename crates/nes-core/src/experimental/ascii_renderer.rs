//! Experimental ASCII renderer for the NES framebuffer.
//!
//! Converts raw RGBA frame data into an ASCII string representation.

#[cfg(feature = "nova")]
use crate::{FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};

#[cfg(feature = "nova")]
/// Converts an RGBA framebuffer to an ASCII art string.
pub struct AsciiRenderer {
    chars: &'static [char],
}

#[cfg(feature = "nova")]
impl Default for AsciiRenderer {
    fn default() -> Self {
        Self {
            chars: &[' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'],
        }
    }
}

#[cfg(feature = "nova")]
impl AsciiRenderer {
    /// Creates a new `AsciiRenderer` with the given character set (ordered from dark to light).
    #[must_use]
    pub fn new(chars: &'static [char]) -> Self {
        Self { chars }
    }

    /// Renders the given RGBA framebuffer to an ASCII string.
    #[must_use]
    pub fn render(&self, framebuffer: &[u8], scale: usize) -> String {
        if scale == 0 || framebuffer.len() < FRAME_RGBA_BYTES {
            return String::new();
        }

        let width = FRAME_WIDTH / scale;
        let height = FRAME_HEIGHT / scale;

        let mut output = String::with_capacity((width + 1) * height);
        let num_chars = self.chars.len() as u32;

        for y in 0..height {
            for x in 0..width {
                // Determine luminosity of this block
                let mut sum_r: u32 = 0;
                let mut sum_g: u32 = 0;
                let mut sum_b: u32 = 0;

                for dy in 0..scale {
                    for dx in 0..scale {
                        let py = (y * scale + dy) * FRAME_WIDTH;
                        let px = x * scale + dx;
                        let idx = (py + px) * 4;

                        sum_r += framebuffer[idx] as u32;
                        sum_g += framebuffer[idx + 1] as u32;
                        sum_b += framebuffer[idx + 2] as u32;
                    }
                }

                let pixels = (scale * scale) as u32;
                let avg_r = sum_r / pixels;
                let avg_g = sum_g / pixels;
                let avg_b = sum_b / pixels;

                // standard luminosity weights
                let luma = (avg_r * 299 + avg_g * 587 + avg_b * 114) / 1000;

                // map 0-255 to char index
                let mut char_idx = (luma * num_chars / 256) as usize;
                if char_idx >= num_chars as usize {
                    char_idx = num_chars as usize - 1;
                }

                output.push(self.chars[char_idx]);
            }
            if y < height - 1 {
                output.push('\n');
            }
        }

        output
    }
}

#[cfg(feature = "nova")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_renderer_empty() {
        let renderer = AsciiRenderer::default();
        assert_eq!(renderer.render(&[], 1), "");
    }

    #[test]
    fn test_ascii_renderer_scale() {
        let renderer = AsciiRenderer::default();
        let mut fb = vec![0; FRAME_RGBA_BYTES];

        // Fill first pixel block with white so first char is '@'
        for y in 0..8 {
            for x in 0..8 {
                let idx = (y * FRAME_WIDTH + x) * 4;
                fb[idx] = 255;
                fb[idx + 1] = 255;
                fb[idx + 2] = 255;
                fb[idx + 3] = 255;
            }
        }

        let out = renderer.render(&fb, 8);
        // width = 256 / 8 = 32
        // height = 240 / 8 = 30
        assert_eq!(out.lines().count(), 30);
        assert!(out.starts_with('@')); // white is the last char
    }
}
