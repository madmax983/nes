//! Experimental ASCII renderer for the NES framebuffer.
//!
//! This module provides a way to convert the raw RGBA output
//! of the PPU into an ASCII art string for terminal display.

#[cfg(feature = "nova")]
/// A stateless renderer that converts an RGBA framebuffer to ASCII.
pub struct AsciiRenderer;

#[cfg(feature = "nova")]
impl AsciiRenderer {
    /// Renders an RGBA framebuffer as an ASCII art string.
    ///
    /// The input buffer should be in `RGBA8` format (4 bytes per pixel).
    /// `width` and `height` define the dimensions of the framebuffer.
    /// `scale_x` and `scale_y` allow downscaling the output.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::ascii_renderer::AsciiRenderer;
    /// let framebuffer = vec![
    ///     255, 255, 255, 255, 0, 0, 0, 255,
    ///     0, 0, 0, 255, 255, 255, 255, 255,
    /// ];
    /// let ascii = AsciiRenderer::render(&framebuffer, 2, 2, 1, 1);
    /// assert_eq!(ascii, "@ \n @\n");
    /// ```
    pub fn render(
        framebuffer: &[u8],
        width: usize,
        height: usize,
        scale_x: usize,
        scale_y: usize,
    ) -> String {
        let mut output = String::new();

        // 10 levels of characters
        let chars = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

        for y in (0..height).step_by(scale_y) {
            for x in (0..width).step_by(scale_x) {
                // Calculate average luma for the block
                let mut total_luma = 0;
                let mut count = 0;

                for by in 0..scale_y {
                    for bx in 0..scale_x {
                        let py = y + by;
                        let px = x + bx;

                        if py < height && px < width {
                            let idx = (py * width + px) * 4;
                            if idx + 2 < framebuffer.len() {
                                let r = u32::from(framebuffer[idx]);
                                let g = u32::from(framebuffer[idx + 1]);
                                let b = u32::from(framebuffer[idx + 2]);

                                // Luma = 0.299R + 0.587G + 0.114B
                                let luma = (r * 77 + g * 150 + b * 29) >> 8;
                                total_luma += luma;
                                count += 1;
                            }
                        }
                    }
                }

                let avg_luma = if count > 0 { total_luma / count } else { 0 };

                let char_idx = ((avg_luma * 9) / 255) as usize;
                output.push(chars[char_idx.clamp(0, 9)]);
            }
            output.push('\n');
        }

        output
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn render_black_framebuffer_returns_spaces() {
        let framebuffer = vec![0; 4 * 2 * 2]; // 2x2 black pixels
        let ascii = AsciiRenderer::render(&framebuffer, 2, 2, 1, 1);
        assert_eq!(ascii, "  \n  \n");
    }

    #[test]
    fn render_white_framebuffer_returns_ats() {
        let framebuffer = vec![255; 4 * 2 * 2]; // 2x2 white pixels
        let ascii = AsciiRenderer::render(&framebuffer, 2, 2, 1, 1);
        assert_eq!(ascii, "@@\n@@\n");
    }

    #[test]
    fn render_downscales_correctly() {
        let framebuffer = vec![
            255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255,
            255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255,
        ];
        // 2x2 grid downscaled by 2x2 should yield a single character based on avg luma
        // Top-left block: 2 white, 2 black -> avg luma = 127
        // (127 * 9) / 255 = 4 -> '='
        let ascii = AsciiRenderer::render(&framebuffer, 2, 2, 2, 2);
        assert_eq!(ascii, "=\n");
    }
}
