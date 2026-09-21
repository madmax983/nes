//! Experimental CRT-style post-processing filter.
//!
//! This module provides the `CrtFilter` which applies scanlines and
//! chromatic aberration to the raw RGBA framebuffer to simulate a retro TV.

use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

/// A stateless filter that applies CRT effects to an RGBA framebuffer.
pub struct CrtFilter;

impl CrtFilter {
    /// Applies scanlines and chromatic aberration to an RGBA framebuffer.
    ///
    /// The input buffer must be in `RGBA8` format with a length of `FRAME_WIDTH * FRAME_HEIGHT * 4`.
    ///
    /// `scanline_intensity` ranges from 0 (no effect) to 255 (black scanlines).
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::crt_filter::CrtFilter;
    /// # use nes_core::constants::FRAME_RGBA_BYTES;
    /// let mut frame = vec![255; FRAME_RGBA_BYTES];
    /// CrtFilter::apply_crt(&mut frame, 128);
    /// ```
    pub fn apply_crt(framebuffer: &mut [u8], scanline_intensity: u8) {
        if framebuffer.len() != FRAME_WIDTH * FRAME_HEIGHT * 4 {
            return;
        }

        let width = FRAME_WIDTH;
        let height = FRAME_HEIGHT;
        let scanline_multiplier = 255u32.saturating_sub(u32::from(scanline_intensity));

        // We will process row by row
        // To avoid allocating a new buffer, we apply chromatic aberration in place by shifting.
        // Chromatic Aberration: R shifted left, B shifted right.

        // First pass: Chromatic Aberration (horizontal shift)
        for y in 0..height {
            let row_start = y * width * 4;
            // Shift R left by 1 pixel (read from right)
            for x in 0..(width - 1) {
                let idx = row_start + x * 4;
                let next_idx = idx + 4;
                framebuffer[idx] = framebuffer[next_idx]; // R
            }
            // Shift B right by 1 pixel (read from left)
            for x in (1..width).rev() {
                let idx = row_start + x * 4;
                let prev_idx = idx - 4;
                framebuffer[idx + 2] = framebuffer[prev_idx + 2]; // B
            }
        }

        // Second pass: Scanlines
        // Darken every odd row
        if scanline_intensity > 0 {
            for y in (1..height).step_by(2) {
                let row_start = y * width * 4;
                for x in 0..width {
                    let idx = row_start + x * 4;
                    let r = (u32::from(framebuffer[idx]) * scanline_multiplier / 255) as u8;
                    let g = (u32::from(framebuffer[idx + 1]) * scanline_multiplier / 255) as u8;
                    let b = (u32::from(framebuffer[idx + 2]) * scanline_multiplier / 255) as u8;

                    framebuffer[idx] = r;
                    framebuffer[idx + 1] = g;
                    framebuffer[idx + 2] = b;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::FRAME_RGBA_BYTES;

    #[test]
    fn apply_crt_aberration_shifts_colors() {
        let mut frame = vec![0; FRAME_RGBA_BYTES];

        // Put a white pixel in the middle
        let middle_idx = (FRAME_HEIGHT / 2 * FRAME_WIDTH + FRAME_WIDTH / 2) * 4;
        frame[middle_idx] = 255;
        frame[middle_idx + 1] = 255;
        frame[middle_idx + 2] = 255;

        CrtFilter::apply_crt(&mut frame, 0);

        // R should be shifted left (so the pixel at middle - 1 should have R=255)
        assert_eq!(frame[middle_idx - 4], 255); // R

        // B should be shifted right (so pixel at middle + 1 should have B=255)
        assert_eq!(frame[middle_idx + 4 + 2], 255); // B
    }

    #[test]
    fn apply_crt_scanlines_darken_odd_lines() {
        let mut frame = vec![255; FRAME_RGBA_BYTES];

        CrtFilter::apply_crt(&mut frame, 128);

        let row0_idx = 0;
        let row1_idx = FRAME_WIDTH * 4;

        // Row 0 is even, so it shouldn't be darkened by scanlines.
        // However, chromatic aberration shifts things. For a solid color, it doesn't matter.
        assert_eq!(frame[row0_idx + 1], 255); // G

        // Row 1 is odd, so it should be darkened.
        // scanline_multiplier = 255 - 128 = 127
        // 255 * 127 / 255 = 127
        assert_eq!(frame[row1_idx + 1], 127); // G
    }

    #[test]
    fn ignores_invalid_framebuffer() {
        let mut frame = vec![255, 255, 255]; // Missing alpha, wrong length
        CrtFilter::apply_crt(&mut frame, 128);
        assert_eq!(frame, vec![255, 255, 255]);
    }
}
