//! Experimental ASCII renderer for the NES framebuffer.
//!
//! Converts raw RGBA framebuffers into ASCII art, useful for text-mode interfaces.

#[cfg(feature = "nova")]
pub struct AsciiRenderer;

#[cfg(feature = "nova")]
impl AsciiRenderer {
    /// The default ASCII characters used for intensity mapping, from darkest to lightest.
    pub const DEFAULT_PALETTE: &'static [u8] = b" .:-=+*#%@";

    /// Converts an RGBA framebuffer to an ASCII string.
    ///
    /// # Parameters
    ///
    /// * `framebuffer`: Raw RGBA pixel data.
    /// * `width`: Source image width in pixels.
    /// * `height`: Source image height in pixels.
    /// * `block_w`: Width of the block (in pixels) to downsample into a single character.
    /// * `block_h`: Height of the block (in pixels) to downsample into a single character.
    /// * `palette`: A byte slice of ASCII characters ordered from darkest to lightest.
    pub fn render(
        framebuffer: &[u8],
        width: usize,
        height: usize,
        block_w: usize,
        block_h: usize,
        palette: &[u8],
    ) -> String {
        if framebuffer.is_empty()
            || width == 0
            || height == 0
            || block_w == 0
            || block_h == 0
            || palette.is_empty()
        {
            return String::new();
        }

        let out_w = width / block_w;
        let out_h = height / block_h;

        use std::fmt::Write;
        let mut out = String::with_capacity(out_h * (out_w + 1));

        for y in 0..out_h {
            for x in 0..out_w {
                let mut sum_luma = 0u32;
                let mut count = 0u32;

                for dy in 0..block_h {
                    for dx in 0..block_w {
                        let px = x * block_w + dx;
                        let py = y * block_h + dy;
                        if px < width && py < height {
                            let idx = (py * width + px) * 4;
                            if idx + 2 < framebuffer.len() {
                                let r = u32::from(framebuffer[idx]);
                                let g = u32::from(framebuffer[idx + 1]);
                                let b = u32::from(framebuffer[idx + 2]);
                                let luma = (r * 77 + g * 150 + b * 29) >> 8;
                                sum_luma += luma;
                                count += 1;
                            }
                        }
                    }
                }

                let avg_luma = if count > 0 { sum_luma / count } else { 0 };
                let char_idx = (avg_luma * (palette.len() as u32 - 1)) / 255;
                let _ = write!(&mut out, "{}", palette[char_idx as usize] as char);
            }
            let _ = writeln!(&mut out);
        }

        out
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn render_empty_or_invalid() {
        assert_eq!(
            AsciiRenderer::render(&[], 0, 0, 1, 1, AsciiRenderer::DEFAULT_PALETTE),
            ""
        );
    }

    #[test]
    fn render_black_frame() {
        let frame = vec![0; 4 * 4 * 4]; // 4x4 RGBA
        let out = AsciiRenderer::render(&frame, 4, 4, 2, 2, AsciiRenderer::DEFAULT_PALETTE);
        // 4x4 / 2x2 = 2x2 characters
        assert_eq!(out, "  \n  \n"); // index 0 in default palette is ' '
    }

    #[test]
    fn render_white_frame() {
        let frame = vec![255; 4 * 4 * 4]; // 4x4 RGBA
        let out = AsciiRenderer::render(&frame, 4, 4, 2, 2, AsciiRenderer::DEFAULT_PALETTE);
        assert_eq!(out, "@@\n@@\n"); // index 9 in default palette is '@'
    }
}
