#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaMode {
    Normal,
    Grayscale,
    Sepia,
    Invert,
    Gameboy,
}

#[cfg(feature = "nova")]
impl ChromaMode {
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::Grayscale,
            Self::Grayscale => Self::Sepia,
            Self::Sepia => Self::Invert,
            Self::Invert => Self::Gameboy,
            Self::Gameboy => Self::Normal,
        }
    }
}

#[cfg(feature = "nova")]
pub fn apply_filter(mode: ChromaMode, frame: &mut [u8]) {
    if mode == ChromaMode::Normal {
        return;
    }

    // Process every 4 bytes (RGBA)
    for pixel in frame.chunks_exact_mut(4) {
        let r = f32::from(pixel[0]);
        let g = f32::from(pixel[1]);
        let b = f32::from(pixel[2]);
        // pixel[3] is Alpha, unchanged

        match mode {
            ChromaMode::Grayscale => {
                // Luminance formula
                let lum = 0.299 * r + 0.587 * g + 0.114 * b;
                let val = lum.clamp(0.0, 255.0) as u8;
                pixel[0] = val;
                pixel[1] = val;
                pixel[2] = val;
            }
            ChromaMode::Sepia => {
                let out_r = (0.393 * r + 0.769 * g + 0.189 * b).clamp(0.0, 255.0) as u8;
                let out_g = (0.349 * r + 0.686 * g + 0.168 * b).clamp(0.0, 255.0) as u8;
                let out_b = (0.272 * r + 0.534 * g + 0.131 * b).clamp(0.0, 255.0) as u8;
                pixel[0] = out_r;
                pixel[1] = out_g;
                pixel[2] = out_b;
            }
            ChromaMode::Invert => {
                pixel[0] = 255 - pixel[0];
                pixel[1] = 255 - pixel[1];
                pixel[2] = 255 - pixel[2];
            }
            ChromaMode::Gameboy => {
                // Approximate mapping to a 4-color Gameboy palette
                let lum = 0.299 * r + 0.587 * g + 0.114 * b;

                let (out_r, out_g, out_b) = if lum < 64.0 {
                    (15, 56, 15) // Darkest green
                } else if lum < 128.0 {
                    (48, 98, 48) // Dark green
                } else if lum < 192.0 {
                    (139, 172, 15) // Light green
                } else {
                    (155, 188, 15) // Lightest green
                };

                pixel[0] = out_r;
                pixel[1] = out_g;
                pixel[2] = out_b;
            }
            ChromaMode::Normal => unreachable!(),
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn chroma_mode_cycles_correctly() {
        let mode = ChromaMode::Normal;
        let mode = mode.next();
        assert_eq!(mode, ChromaMode::Grayscale);
        let mode = mode.next();
        assert_eq!(mode, ChromaMode::Sepia);
        let mode = mode.next();
        assert_eq!(mode, ChromaMode::Invert);
        let mode = mode.next();
        assert_eq!(mode, ChromaMode::Gameboy);
        let mode = mode.next();
        assert_eq!(mode, ChromaMode::Normal);
    }

    #[test]
    fn apply_filter_normal_does_nothing() {
        let mut frame = [255, 0, 0, 255]; // Pure red
        apply_filter(ChromaMode::Normal, &mut frame);
        assert_eq!(frame, [255, 0, 0, 255]);
    }

    #[test]
    fn apply_filter_grayscale_converts_to_gray() {
        let mut frame = [255, 0, 0, 255]; // Pure red
        apply_filter(ChromaMode::Grayscale, &mut frame);
        // Red luminance: 255 * 0.299 = 76.245 -> 76
        assert_eq!(frame, [76, 76, 76, 255]);
    }

    #[test]
    fn apply_filter_sepia_applies_matrix() {
        let mut frame = [100, 100, 100, 255]; // Gray
        apply_filter(ChromaMode::Sepia, &mut frame);
        // Sepia for gray (100):
        // R = (0.393 + 0.769 + 0.189) * 100 = 135.1
        // G = (0.349 + 0.686 + 0.168) * 100 = 120.3
        // B = (0.272 + 0.534 + 0.131) * 100 = 93.7
        assert_eq!(frame, [135, 120, 93, 255]);
    }

    #[test]
    fn apply_filter_invert_inverts_rgb() {
        let mut frame = [200, 50, 25, 255]; // Random color
        apply_filter(ChromaMode::Invert, &mut frame);
        assert_eq!(frame, [55, 205, 230, 255]);
    }

    #[test]
    fn apply_filter_gameboy_maps_to_palette() {
        let mut frame = [255, 255, 255, 255]; // White (lum = 255)
        apply_filter(ChromaMode::Gameboy, &mut frame);
        // Should map to the lightest green
        assert_eq!(frame, [155, 188, 15, 255]);

        let mut frame2 = [0, 0, 0, 255]; // Black (lum = 0)
        apply_filter(ChromaMode::Gameboy, &mut frame2);
        // Should map to darkest green
        assert_eq!(frame2, [15, 56, 15, 255]);
    }

    #[test]
    fn apply_filter_handles_multiple_pixels() {
        let mut frame = [
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
        ];
        apply_filter(ChromaMode::Invert, &mut frame);
        assert_eq!(
            frame,
            [0, 255, 255, 255, 255, 0, 255, 255, 255, 255, 0, 255,]
        );
    }
}
