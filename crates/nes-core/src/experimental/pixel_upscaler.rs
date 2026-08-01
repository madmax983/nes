//! Experimental pixel art upscaling.
//!
//! This module provides the `PixelUpscaler`, an experimental utility that applies
//! algorithms like Scale2x to the raw NES framebuffer to produce higher-resolution
//! pixel art without the blurriness of bilinear filtering.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
/// An upscaler that applies the Scale2x algorithm to a raw RGBA framebuffer.
pub struct PixelUpscaler;

#[cfg(feature = "nova")]
impl PixelUpscaler {
    /// Upscales a standard NES framebuffer (256x240) to 2x resolution (512x480) using Scale2x.
    ///
    /// The input buffer must be exactly `FRAME_WIDTH * FRAME_HEIGHT * 4` bytes.
    /// The returned vector will be exactly `(FRAME_WIDTH * 2) * (FRAME_HEIGHT * 2) * 4` bytes.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::pixel_upscaler::PixelUpscaler;
    /// # use nes_core::constants::{FRAME_WIDTH, FRAME_HEIGHT, FRAME_RGBA_BYTES};
    /// let mut frame = vec![0u8; FRAME_RGBA_BYTES];
    /// let upscaled = PixelUpscaler::scale2x(&frame).unwrap();
    /// assert_eq!(upscaled.len(), FRAME_RGBA_BYTES * 4);
    /// ```
    pub fn scale2x(input: &[u8]) -> Result<Vec<u8>, &'static str> {
        let expected_len = FRAME_WIDTH * FRAME_HEIGHT * 4;
        if input.len() != expected_len {
            return Err("Input buffer length does not match standard NES framebuffer size");
        }

        let out_width = FRAME_WIDTH * 2;
        let out_height = FRAME_HEIGHT * 2;
        let mut output = vec![0u8; out_width * out_height * 4];

        for y in 0..FRAME_HEIGHT {
            for x in 0..FRAME_WIDTH {
                let get_pixel = |cx: usize, cy: usize| {
                    let clamped_x = cx.min(FRAME_WIDTH - 1);
                    let clamped_y = cy.min(FRAME_HEIGHT - 1);
                    let idx = (clamped_y * FRAME_WIDTH + clamped_x) * 4;
                    u32::from_ne_bytes([input[idx], input[idx + 1], input[idx + 2], input[idx + 3]])
                };

                let p = get_pixel(x, y);
                let a = get_pixel(x, y.saturating_sub(1));
                let b = get_pixel(x + 1, y);
                let c = get_pixel(x.saturating_sub(1), y);
                let d = get_pixel(x, y + 1);

                let mut e0 = p;
                let mut e1 = p;
                let mut e2 = p;
                let mut e3 = p;

                if c == a && c != d && a != b {
                    e0 = a;
                }
                if a == b && a != c && b != d {
                    e1 = b;
                }
                if d == c && d != b && c != a {
                    e2 = c;
                }
                if b == d && b != a && d != c {
                    e3 = d;
                }

                let mut write_pixel = |cx: usize, cy: usize, val: u32| {
                    let idx = (cy * out_width + cx) * 4;
                    let bytes = val.to_ne_bytes();
                    output[idx] = bytes[0];
                    output[idx + 1] = bytes[1];
                    output[idx + 2] = bytes[2];
                    output[idx + 3] = bytes[3];
                };

                let out_x = x * 2;
                let out_y = y * 2;
                write_pixel(out_x, out_y, e0);
                write_pixel(out_x + 1, out_y, e1);
                write_pixel(out_x, out_y + 1, e2);
                write_pixel(out_x + 1, out_y + 1, e3);
            }
        }

        Ok(output)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::constants::FRAME_RGBA_BYTES;

    #[test]
    fn scale2x_returns_error_on_invalid_size() {
        let result = PixelUpscaler::scale2x(&[0u8; 100]);
        assert!(result.is_err());
    }

    #[test]
    fn scale2x_produces_correct_output_size() {
        let frame = vec![0u8; FRAME_RGBA_BYTES];
        let upscaled = PixelUpscaler::scale2x(&frame).unwrap();
        assert_eq!(upscaled.len(), FRAME_RGBA_BYTES * 4);
    }
}
