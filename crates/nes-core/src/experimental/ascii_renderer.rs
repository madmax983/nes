//! Experimental ASCII art exporter for the NES framebuffer.
//!
//! This module provides utilities to downscale and convert the raw RGBA
//! framebuffer into an ASCII representation, allowing the emulator's output
//! to be visualized in a pure text console.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
const ASCII_RAMP: &[u8] = b" .:-=+*#%@";

/// A stateless utility for rendering raw framebuffers as ASCII art.
#[cfg(feature = "nova")]
pub struct AsciiRenderer;

#[cfg(feature = "nova")]
impl AsciiRenderer {
    /// Renders the RGBA framebuffer into an ASCII string.
    ///
    /// The `framebuffer` must be exactly `FRAME_WIDTH * FRAME_HEIGHT * 4` bytes.
    /// The image is downscaled by averaging the luminance of pixels within
    /// each `block_width` x `block_height` grid.
    ///
    /// # Panics
    ///
    /// Panics if the `framebuffer` is not the correct size.
    pub fn render(framebuffer: &[u8], block_width: usize, block_height: usize) -> String {
        assert_eq!(framebuffer.len(), FRAME_WIDTH * FRAME_HEIGHT * 4);
        assert!(block_width > 0 && block_height > 0);

        let out_width = FRAME_WIDTH / block_width;
        let out_height = FRAME_HEIGHT / block_height;
        let mut out = String::with_capacity(out_height * (out_width + 1));

        for by in 0..out_height {
            for bx in 0..out_width {
                let mut sum_lum = 0u32;
                let mut count = 0u32;

                for y in 0..block_height {
                    for x in 0..block_width {
                        let px = bx * block_width + x;
                        let py = by * block_height + y;
                        let idx = (py * FRAME_WIDTH + px) * 4;

                        let r = framebuffer[idx] as u32;
                        let g = framebuffer[idx + 1] as u32;
                        let b = framebuffer[idx + 2] as u32;

                        // Rec. 709 luminance approximation
                        let lum = (r * 2126 + g * 7152 + b * 722) / 10000;
                        sum_lum += lum;
                        count += 1;
                    }
                }

                let avg_lum = sum_lum / count;
                // Map 0-255 to 0..ASCII_RAMP.len() - 1
                let ramp_idx = (avg_lum * (ASCII_RAMP.len() as u32 - 1)) / 255;
                let c = ASCII_RAMP[ramp_idx as usize] as char;
                out.push(c);
            }
            out.push('\n');
        }

        out
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn render_panics_on_invalid_framebuffer_size() {
        let frame = vec![0u8; 10];
        AsciiRenderer::render(&frame, 8, 8);
    }

    #[test]
    fn render_produces_ascii_string_on_valid_framebuffer() {
        let frame = vec![255u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
        let result = AsciiRenderer::render(&frame, 8, 8);
        assert!(!result.is_empty());
    }
}
