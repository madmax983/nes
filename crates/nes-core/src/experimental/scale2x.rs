//! Experimental Scale2x algorithm for upscaling pixel art.
//!
//! This module implements the Scale2x algorithm, originally developed by Andrea Mazzoleni
//! for AdvanceMAME. It takes a raw RGBA framebuffer and scales it to exactly 2x the original
//! size without introducing blurring.

#[cfg(feature = "nova")]
/// A stateless upscaler that applies the Scale2x algorithm to an RGBA framebuffer.
pub struct Scale2x;

#[cfg(feature = "nova")]
impl Scale2x {
    /// Scales a raw RGBA framebuffer by 2x.
    ///
    /// `src` must be an RGBA byte slice of exactly `width * height * 4` bytes.
    /// The returned vector will be exactly `(width * 2) * (height * 2) * 4` bytes.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::scale2x::Scale2x;
    /// let frame = vec![0; 4 * 4 * 4]; // 4x4 image
    /// let scaled = Scale2x::scale(&frame, 4, 4);
    /// assert_eq!(scaled.len(), 8 * 8 * 4); // 8x8 image
    /// ```
    #[must_use]
    pub fn scale(src: &[u8], width: usize, height: usize) -> Vec<u8> {
        if src.len() != width * height * 4 {
            return vec![];
        }

        let out_width = width * 2;
        let out_height = height * 2;
        let mut dst = vec![0u8; out_width * out_height * 4];

        for y in 0..height {
            for x in 0..width {
                // Read pixel as a 32-bit value to make comparisons easy.
                // The pixel neighbors:
                // E0 E1 E2
                // E3 E4 E5
                // E6 E7 E8
                let get_pixel = |px: isize, py: isize| -> u32 {
                    let clamped_x = px.clamp(0, width as isize - 1) as usize;
                    let clamped_y = py.clamp(0, height as isize - 1) as usize;
                    let idx = (clamped_y * width + clamped_x) * 4;
                    u32::from_le_bytes(src[idx..idx + 4].try_into().unwrap())
                };

                let cx = x as isize;
                let cy = y as isize;

                let e1 = get_pixel(cx, cy - 1);
                let e3 = get_pixel(cx - 1, cy);
                let e4 = get_pixel(cx, cy);
                let e5 = get_pixel(cx + 1, cy);
                let e7 = get_pixel(cx, cy + 1);

                // Scale2x logic
                let e40 = if e1 != e7 && e3 != e5 && e3 == e1 {
                    e3
                } else {
                    e4
                };
                let e41 = if e1 != e7 && e3 != e5 && e5 == e1 {
                    e5
                } else {
                    e4
                };
                let e42 = if e1 != e7 && e3 != e5 && e3 == e7 {
                    e3
                } else {
                    e4
                };
                let e43 = if e1 != e7 && e3 != e5 && e5 == e7 {
                    e5
                } else {
                    e4
                };

                let mut set_pixel = |dx: usize, dy: usize, color: u32| {
                    let idx = (dy * out_width + dx) * 4;
                    dst[idx..idx + 4].copy_from_slice(&color.to_le_bytes());
                };

                set_pixel(x * 2, y * 2, e40);
                set_pixel(x * 2 + 1, y * 2, e41);
                set_pixel(x * 2, y * 2 + 1, e42);
                set_pixel(x * 2 + 1, y * 2 + 1, e43);
            }
        }
        dst
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_scale2x_dimensions() {
        let src = vec![255; 4 * 4 * 4]; // 4x4 RGBA
        let dst = Scale2x::scale(&src, 4, 4);
        assert_eq!(dst.len(), (4 * 2) * (4 * 2) * 4); // 8x8 RGBA
    }

    #[test]
    fn test_scale2x_invalid_len() {
        let src = vec![0; 10]; // Not enough for a 4x4 RGBA
        let dst = Scale2x::scale(&src, 4, 4);
        assert_eq!(dst.len(), 0);
    }

    fn set_pixel(src: &mut [u8], x: usize, y: usize, color: u8) {
        let idx = (y * 3 + x) * 4;
        src[idx] = color;
        src[idx + 1] = color;
        src[idx + 2] = color;
        src[idx + 3] = 255;
    }

    #[test]
    fn test_scale2x_all_corners() {
        let mut src = vec![0; 3 * 3 * 4];

        let get_dst = |dst: &[u8], x: usize, y: usize| -> u8 {
            let idx = (y * 6 + x) * 4;
            dst[idx]
        };

        set_pixel(&mut src, 1, 1, 100); // center (e4)

        // Case 1: Top-Left (e1 == e3)
        set_pixel(&mut src, 1, 0, 10); // top
        set_pixel(&mut src, 0, 1, 10); // left
        set_pixel(&mut src, 2, 1, 20); // right
        set_pixel(&mut src, 1, 2, 30); // bottom
        let dst = Scale2x::scale(&src, 3, 3);
        assert_eq!(get_dst(&dst, 2, 2), 10); // e40
        assert_eq!(get_dst(&dst, 3, 2), 100); // e41

        // Case 2: Top-Right (e1 == e5)
        set_pixel(&mut src, 1, 0, 10); // top
        set_pixel(&mut src, 0, 1, 20); // left
        set_pixel(&mut src, 2, 1, 10); // right
        set_pixel(&mut src, 1, 2, 30); // bottom
        let dst = Scale2x::scale(&src, 3, 3);
        assert_eq!(get_dst(&dst, 3, 2), 10); // e41
        assert_eq!(get_dst(&dst, 2, 2), 100); // e40

        // Case 3: Bottom-Left (e7 == e3)
        set_pixel(&mut src, 1, 0, 20); // top
        set_pixel(&mut src, 0, 1, 10); // left
        set_pixel(&mut src, 2, 1, 30); // right
        set_pixel(&mut src, 1, 2, 10); // bottom
        let dst = Scale2x::scale(&src, 3, 3);
        assert_eq!(get_dst(&dst, 2, 3), 10); // e42
        assert_eq!(get_dst(&dst, 3, 3), 100); // e43

        // Case 4: Bottom-Right (e7 == e5)
        set_pixel(&mut src, 1, 0, 20); // top
        set_pixel(&mut src, 0, 1, 30); // left
        set_pixel(&mut src, 2, 1, 10); // right
        set_pixel(&mut src, 1, 2, 10); // bottom
        let dst = Scale2x::scale(&src, 3, 3);
        assert_eq!(get_dst(&dst, 3, 3), 10); // e43
        assert_eq!(get_dst(&dst, 2, 3), 100); // e42
    }
}
