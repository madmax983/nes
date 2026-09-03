//! Experimental post-processing filters for the NES framebuffer.
//!
//! This module provides a way to apply visual themes to the raw RGBA output
//! of the PPU without modifying the core rendering logic or emulator accuracy.

#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Available visual themes that can be applied to the framebuffer.
pub enum Theme {
    /// Standard luminosity-based grayscale conversion.
    Grayscale,
    /// Classic pea-green 4-shade LCD aesthetic.
    Gameboy,
    /// Warm, vintage photograph effect.
    Sepia,
    /// High-contrast red-only display, reminiscent of the Virtual Boy.
    VirtualBoy,
}

#[cfg(feature = "nova")]
/// A stateless filter that modifies an RGBA framebuffer in place.
pub struct ThemeFilter;

#[cfg(feature = "nova")]
impl ThemeFilter {
    /// Applies a visual [`Theme`] to a raw RGBA framebuffer.
    ///
    /// The input buffer should be in `RGBA8` format (4 bytes per pixel).
    /// The alpha channel is preserved.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::theme_filter::{ThemeFilter, Theme};
    /// let mut framebuffer = vec![255, 100, 50, 255]; // Reddish pixel
    /// ThemeFilter::apply_theme(&mut framebuffer, Theme::Grayscale);
    /// assert_eq!(framebuffer[0], framebuffer[1]); // R == G
    /// assert_eq!(framebuffer[1], framebuffer[2]); // G == B
    /// ```
    pub fn apply_theme(framebuffer: &mut [u8], theme: Theme) {
        // Ensure the buffer is a valid RGBA buffer
        #[allow(clippy::manual_is_multiple_of)]
        if framebuffer.len() % 4 != 0 {
            return;
        }

        match theme {
            Theme::Grayscale => {
                for pixel in framebuffer.as_chunks_mut::<4>().0 {
                    let r = u32::from(pixel[0]);
                    let g = u32::from(pixel[1]);
                    let b = u32::from(pixel[2]);

                    // Standard luminosity (Luma = 0.299R + 0.587G + 0.114B)
                    // Approximated with integer math: (77*R + 150*G + 29*B) >> 8
                    let luma = (r * 77 + g * 150 + b * 29) >> 8;
                    let luma = luma as u8;

                    pixel[0] = luma;
                    pixel[1] = luma;
                    pixel[2] = luma;
                }
            }
            Theme::Gameboy => {
                // Gameboy greens:
                // Darkest:  (15, 56, 15)
                // Dark:     (48, 98, 48)
                // Light:    (139, 172, 15)
                // Lightest: (155, 188, 15)
                for pixel in framebuffer.as_chunks_mut::<4>().0 {
                    let r = u32::from(pixel[0]);
                    let g = u32::from(pixel[1]);
                    let b = u32::from(pixel[2]);

                    let luma = (r * 77 + g * 150 + b * 29) >> 8;

                    let (out_r, out_g, out_b) = if luma < 64 {
                        (15, 56, 15)
                    } else if luma < 128 {
                        (48, 98, 48)
                    } else if luma < 192 {
                        (139, 172, 15)
                    } else {
                        (155, 188, 15)
                    };

                    pixel[0] = out_r;
                    pixel[1] = out_g;
                    pixel[2] = out_b;
                }
            }
            Theme::Sepia => {
                for pixel in framebuffer.as_chunks_mut::<4>().0 {
                    let r = f32::from(pixel[0]);
                    let g = f32::from(pixel[1]);
                    let b = f32::from(pixel[2]);

                    let tr = (r * 0.393) + (g * 0.769) + (b * 0.189);
                    let tg = (r * 0.349) + (g * 0.686) + (b * 0.168);
                    let tb = (r * 0.272) + (g * 0.534) + (b * 0.131);

                    pixel[0] = tr.min(255.0) as u8;
                    pixel[1] = tg.min(255.0) as u8;
                    pixel[2] = tb.min(255.0) as u8;
                }
            }
            Theme::VirtualBoy => {
                for pixel in framebuffer.as_chunks_mut::<4>().0 {
                    let r = u32::from(pixel[0]);
                    let g = u32::from(pixel[1]);
                    let b = u32::from(pixel[2]);

                    let luma = (r * 77 + g * 150 + b * 29) >> 8;

                    // Virtual Boy has pure red with varying intensity
                    pixel[0] = luma.min(255) as u8;
                    pixel[1] = 0;
                    pixel[2] = 0;
                }
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn grayscale_theme_converts_rgb_to_equal_values() {
        let mut frame = vec![255, 0, 0, 255, 0, 255, 0, 255];
        ThemeFilter::apply_theme(&mut frame, Theme::Grayscale);

        // Pixel 1: Pure Red
        assert_eq!(frame[0], frame[1]);
        assert_eq!(frame[1], frame[2]);
        assert_eq!(frame[3], 255); // Alpha preserved

        // Pixel 2: Pure Green
        assert_eq!(frame[4], frame[5]);
        assert_eq!(frame[5], frame[6]);
        assert_eq!(frame[7], 255); // Alpha preserved
    }

    #[test]
    fn gameboy_theme_uses_specific_palette() {
        let mut frame = vec![0, 0, 0, 255, 255, 255, 255, 255];
        ThemeFilter::apply_theme(&mut frame, Theme::Gameboy);

        // Pixel 1: Black -> Darkest Green
        assert_eq!(frame[0], 15);
        assert_eq!(frame[1], 56);
        assert_eq!(frame[2], 15);

        // Pixel 2: White -> Lightest Green
        assert_eq!(frame[4], 155);
        assert_eq!(frame[5], 188);
        assert_eq!(frame[6], 15);
    }

    #[test]
    fn virtualboy_theme_leaves_only_red_channel() {
        let mut frame = vec![100, 150, 200, 255];
        ThemeFilter::apply_theme(&mut frame, Theme::VirtualBoy);

        assert!(frame[0] > 0); // Luma applied to red
        assert_eq!(frame[1], 0); // Green cleared
        assert_eq!(frame[2], 0); // Blue cleared
    }

    #[test]
    fn sepia_theme_modifies_colors() {
        let mut frame = vec![100, 150, 200, 255];
        ThemeFilter::apply_theme(&mut frame, Theme::Sepia);

        // Ensure some color transformation happened
        assert!(frame[0] != 100 || frame[1] != 150 || frame[2] != 200);
        assert_eq!(frame[3], 255);
    }

    #[test]
    fn ignores_invalid_framebuffer() {
        let mut frame = vec![255, 255, 255]; // Missing alpha, len 3
        ThemeFilter::apply_theme(&mut frame, Theme::Grayscale);

        // Should not panic and should leave buffer unchanged
        assert_eq!(frame, vec![255, 255, 255]);
    }
}
