//! Experimental ASCII art exporter for the NES framebuffer.
//!
//! This module provides an `AsciiExporter` that takes a raw RGBA framebuffer
//! and downsamples it into an ASCII string using a configurable terminal grid
//! size. It maps pixel luminance to an ASCII character palette.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
/// Configuration for the ASCII exporter.
pub struct AsciiExporter {
    width: usize,
    height: usize,
    palette: &'static [u8],
}

#[cfg(feature = "nova")]
impl Default for AsciiExporter {
    fn default() -> Self {
        Self {
            width: 64,
            height: 30,
            palette: b" .:-=+*#%@",
        }
    }
}

#[cfg(feature = "nova")]
impl AsciiExporter {
    /// Creates a new `AsciiExporter` with default settings (64x30 grid).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the target grid size for the ASCII output.
    pub fn with_size(mut self, width: usize, height: usize) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Sets the character palette to use, from darkest to lightest.
    pub fn with_palette(mut self, palette: &'static [u8]) -> Self {
        self.palette = palette;
        self
    }

    /// Converts a raw RGBA framebuffer into an ASCII string.
    ///
    /// The input buffer must match `FRAME_WIDTH * FRAME_HEIGHT * 4` bytes.
    #[must_use]
    pub fn render_frame(&self, framebuffer: &[u8]) -> String {
        let expected_len = FRAME_WIDTH * FRAME_HEIGHT * 4;
        if framebuffer.len() != expected_len {
            return String::new(); // Invalid buffer
        }

        let mut output = String::with_capacity((self.width + 1) * self.height);

        let block_width = FRAME_WIDTH as f32 / self.width as f32;
        let block_height = FRAME_HEIGHT as f32 / self.height as f32;

        for grid_y in 0..self.height {
            for grid_x in 0..self.width {
                // Determine source coordinates for nearest-neighbor downsampling
                let src_x = (grid_x as f32 * block_width) as usize;
                let src_y = (grid_y as f32 * block_height) as usize;

                let pixel_idx = (src_y * FRAME_WIDTH + src_x) * 4;
                let r = u32::from(framebuffer[pixel_idx]);
                let g = u32::from(framebuffer[pixel_idx + 1]);
                let b = u32::from(framebuffer[pixel_idx + 2]);

                // Calculate luminance (Luma = 0.299R + 0.587G + 0.114B)
                let luma = (r * 77 + g * 150 + b * 29) >> 8;

                // Map luminance to palette
                let palette_idx = (luma * self.palette.len() as u32) / 256;
                let palette_idx = palette_idx.min((self.palette.len() - 1) as u32) as usize;

                output.push(self.palette[palette_idx] as char);
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
    fn default_config_is_64x30() {
        let exporter = AsciiExporter::new();
        assert_eq!(exporter.width, 64);
        assert_eq!(exporter.height, 30);
    }

    #[test]
    fn handles_invalid_buffer() {
        let exporter = AsciiExporter::new();
        let result = exporter.render_frame(&[0; 100]);
        assert_eq!(result, "");
    }

    #[test]
    fn renders_black_frame() {
        let exporter = AsciiExporter::new().with_size(4, 2);
        let frame = vec![0; FRAME_WIDTH * FRAME_HEIGHT * 4];
        let result = exporter.render_frame(&frame);

        // Palette starts with ' ' (space) for 0 luminance
        let expected = "    \n    \n";
        assert_eq!(result, expected);
    }

    #[test]
    fn renders_white_frame() {
        let exporter = AsciiExporter::new().with_size(4, 2);
        let frame = vec![255; FRAME_WIDTH * FRAME_HEIGHT * 4];
        let result = exporter.render_frame(&frame);

        // Palette ends with '@' for max luminance
        let expected = "@@@@\n@@@@\n";
        assert_eq!(result, expected);
    }
}
