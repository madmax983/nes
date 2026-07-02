//! Experimental tool to render the NES framebuffer into ANSI terminal characters.
//!
//! This module provides a way to output the emulator's visual state directly into a
//! terminal using ANSI escape codes, without requiring an external UI library.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
/// A utility for converting an NES framebuffer into a string of ANSI escape codes.
pub struct AnsiRenderer;

#[cfg(feature = "nova")]
impl AnsiRenderer {
    /// Renders the emulator's current framebuffer into a printable ANSI string.
    ///
    /// It uses a half-block character (▀) where the foreground color represents the
    /// top pixel and the background color represents the bottom pixel, effectively
    /// halving the vertical resolution to better fit typical terminal character aspect ratios.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_core::NesCore;
    /// use nes_core::experimental::ansi_renderer::AnsiRenderer;
    ///
    /// let core = NesCore::new();
    /// let ansi_string = AnsiRenderer::render_framebuffer(&core);
    /// // ansi_string can now be printed directly to stdout
    /// ```
    pub fn render_framebuffer(core: &NesCore) -> String {
        let frame = core.framebuffer_rgba();
        Self::render_rgba_slice(&frame, crate::FRAME_WIDTH, crate::FRAME_HEIGHT)
    }

    /// Renders a raw RGBA slice into a printable ANSI string.
    pub fn render_rgba_slice(frame: &[u8], width: usize, height: usize) -> String {
        let mut output = String::with_capacity(width * height * 15);

        for y in (0..height).step_by(2) {
            for x in 0..width {
                let top_idx = (y * width + x) * 4;
                let bottom_idx = ((y + 1) * width + x) * 4;

                let tr = frame[top_idx];
                let tg = frame[top_idx + 1];
                let tb = frame[top_idx + 2];

                let (br, bg, bb) = if y + 1 < height {
                    (
                        frame[bottom_idx],
                        frame[bottom_idx + 1],
                        frame[bottom_idx + 2],
                    )
                } else {
                    (0, 0, 0)
                };

                // \x1b[38;2;R;G;Bm for foreground
                // \x1b[48;2;R;G;Bm for background
                // ▀ is U+2580
                output.push_str(&format!(
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m▀",
                    tr, tg, tb, br, bg, bb
                ));
            }
            // Reset colors and new line
            output.push_str("\x1b[0m\n");
        }

        output
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn test_ansi_render_black_frame() {
        let core = NesCore::new();
        let ansi = AnsiRenderer::render_framebuffer(&core);
        assert!(ansi.contains("\x1b[38;2;0;0;0m\x1b[48;2;0;0;0m▀"));
        assert!(ansi.contains("\x1b[0m\n"));
    }

    #[test]
    fn test_ansi_render_slice() {
        let mut frame = vec![0; 4 * 2 * 2]; // 2x2 image

        // top left: red
        frame[0] = 255;
        frame[1] = 0;
        frame[2] = 0;
        frame[3] = 255;
        // top right: green
        frame[4] = 0;
        frame[5] = 255;
        frame[6] = 0;
        frame[7] = 255;
        // bottom left: blue
        frame[8] = 0;
        frame[9] = 0;
        frame[10] = 255;
        frame[11] = 255;
        // bottom right: white
        frame[12] = 255;
        frame[13] = 255;
        frame[14] = 255;
        frame[15] = 255;

        let ansi = AnsiRenderer::render_rgba_slice(&frame, 2, 2);

        // First pixel: Red over Blue
        assert!(ansi.contains("\x1b[38;2;255;0;0m\x1b[48;2;0;0;255m▀"));
        // Second pixel: Green over White
        assert!(ansi.contains("\x1b[38;2;0;255;0m\x1b[48;2;255;255;255m▀"));
    }
}
