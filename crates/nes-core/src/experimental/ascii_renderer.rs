//! Experimental ASCII art renderer for the NES framebuffer.
//!
//! This module provides a way to convert the raw RGBA output of the PPU
//! into an ASCII string for terminal-based visualization.

use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

/// Converts an RGBA framebuffer into a downscaled ASCII art representation.
pub struct AsciiRenderer;

impl AsciiRenderer {
    /// Renders an RGBA framebuffer into an ASCII art string.
    ///
    /// The output is scaled down to a manageable terminal size. By default, it
    /// converts the 256x240 frame down to a specified `target_width` and `target_height`.
    ///
    /// The characters used are ordered by luminance: ` .:-=+*#%@`
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::ascii_renderer::AsciiRenderer;
    /// # use nes_core::constants::{FRAME_WIDTH, FRAME_HEIGHT, FRAME_RGBA_BYTES};
    /// let framebuffer = vec![0; FRAME_RGBA_BYTES]; // Black screen
    /// let ascii = AsciiRenderer::render(&framebuffer, 64, 30);
    /// assert!(ascii.contains(' ')); // Black pixels render as spaces
    /// ```
    #[must_use]
    pub fn render(framebuffer: &[u8], target_width: usize, target_height: usize) -> String {
        // Validation: must be valid frame buffer size
        if framebuffer.len() != FRAME_WIDTH * FRAME_HEIGHT * 4 {
            return String::new();
        }

        if target_width == 0 || target_height == 0 {
            return String::new();
        }

        let chars = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];
        let mut output = String::with_capacity((target_width + 1) * target_height);

        let scale_x = FRAME_WIDTH as f32 / target_width as f32;
        let scale_y = FRAME_HEIGHT as f32 / target_height as f32;

        for y in 0..target_height {
            for x in 0..target_width {
                // Sample the center of the target cell
                let src_x = ((x as f32 + 0.5) * scale_x) as usize;
                let src_y = ((y as f32 + 0.5) * scale_y) as usize;

                let src_x = src_x.min(FRAME_WIDTH - 1);
                let src_y = src_y.min(FRAME_HEIGHT - 1);

                let pixel_idx = (src_y * FRAME_WIDTH + src_x) * 4;
                let r = framebuffer[pixel_idx] as u32;
                let g = framebuffer[pixel_idx + 1] as u32;
                let b = framebuffer[pixel_idx + 2] as u32;

                // Calculate luma (0-255)
                let luma = (r * 77 + g * 150 + b * 29) >> 8;

                // Map to 0..9 index
                let char_idx = ((luma * 9) / 255) as usize;
                output.push(chars[char_idx.min(9)]);
            }
            output.push('\n');
        }

        output
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::constants::FRAME_RGBA_BYTES;

    #[test]
    fn test_render_black_screen() {
        let framebuffer = vec![0; FRAME_RGBA_BYTES];
        let ascii = AsciiRenderer::render(&framebuffer, 10, 10);

        // Output should be 10 lines of 10 spaces + newline
        let expected_line = "          \n";
        let expected = expected_line.repeat(10);
        assert_eq!(ascii, expected);
    }

    #[test]
    fn test_render_white_screen() {
        let framebuffer = vec![255; FRAME_RGBA_BYTES];
        let ascii = AsciiRenderer::render(&framebuffer, 5, 5);

        let expected_line = "@@@@@\n";
        let expected = expected_line.repeat(5);
        assert_eq!(ascii, expected);
    }

    #[test]
    fn test_invalid_buffer_size() {
        let ascii = AsciiRenderer::render(&[0; 100], 10, 10);
        assert_eq!(ascii, "");
    }
}
