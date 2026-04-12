#[cfg(feature = "nova")]
#[cfg(test)]
mod tests {
    use nes_core::experimental::theme_filter::{Theme, ThemeFilter};

    #[test]
    fn apply_theme_handles_empty_buffer() {
        let mut empty_buffer: [u8; 0] = [];
        ThemeFilter::apply_theme(&mut empty_buffer, Theme::Grayscale);
        assert_eq!(empty_buffer.len(), 0);
    }

    #[test]
    fn apply_theme_ignores_invalid_length_buffer() {
        // Buffer length not divisible by 4
        let mut buffer = [255, 0, 0, 255, 0, 255, 0];
        let original_buffer = buffer;

        ThemeFilter::apply_theme(&mut buffer, Theme::Grayscale);

        // Should return early and not modify buffer
        assert_eq!(buffer, original_buffer);
    }

    #[test]
    fn apply_theme_grayscale() {
        let mut buffer = [255, 0, 0, 255, 0, 255, 0, 128]; // Red pixel, Green pixel

        ThemeFilter::apply_theme(&mut buffer, Theme::Grayscale);

        // Luma for pure red (255, 0, 0): (255 * 77) >> 8 = 76
        assert_eq!(buffer[0], 76);
        assert_eq!(buffer[1], 76);
        assert_eq!(buffer[2], 76);
        assert_eq!(buffer[3], 255); // Alpha preserved

        // Luma for pure green (0, 255, 0): (255 * 150) >> 8 = 149
        assert_eq!(buffer[4], 149);
        assert_eq!(buffer[5], 149);
        assert_eq!(buffer[6], 149);
        assert_eq!(buffer[7], 128); // Alpha preserved
    }

    #[test]
    fn apply_theme_gameboy() {
        let mut buffer = [
            0, 0, 0, 255, // Black (Luma: 0)
            80, 80, 80, 255, // Dark gray (Luma: 80)
            150, 150, 150, 255, // Light gray (Luma: 150)
            255, 255, 255, 255, // White (Luma: 255)
        ];

        ThemeFilter::apply_theme(&mut buffer, Theme::Gameboy);

        // Darkest: (15, 56, 15)
        assert_eq!(&buffer[0..4], &[15, 56, 15, 255]);

        // Dark: (48, 98, 48)
        assert_eq!(&buffer[4..8], &[48, 98, 48, 255]);

        // Light: (139, 172, 15)
        assert_eq!(&buffer[8..12], &[139, 172, 15, 255]);

        // Lightest: (155, 188, 15)
        assert_eq!(&buffer[12..16], &[155, 188, 15, 255]);
    }

    #[test]
    fn apply_theme_sepia() {
        let mut buffer = [200, 150, 100, 255]; // Some base color

        ThemeFilter::apply_theme(&mut buffer, Theme::Sepia);

        // tr = 212, tg = 189, tb = 147
        assert_eq!(buffer[0], 212);
        assert_eq!(buffer[1], 189);
        assert_eq!(buffer[2], 147);
        assert_eq!(buffer[3], 255); // Alpha preserved

        // Test clamping
        let mut clamp_buffer = [255, 255, 255, 255]; // White should clamp to 255
        ThemeFilter::apply_theme(&mut clamp_buffer, Theme::Sepia);
        assert_eq!(clamp_buffer[0], 255);
        assert_eq!(clamp_buffer[1], 255);
        assert_eq!(clamp_buffer[2], 238); // tb = 255 * 0.272 + 255 * 0.534 + 255 * 0.131 = 69.36 + 136.17 + 33.405 = 238.935 -> 238
    }

    #[test]
    fn apply_theme_virtualboy() {
        let mut buffer = [255, 255, 255, 255, 0, 0, 0, 128]; // White pixel, Black pixel

        ThemeFilter::apply_theme(&mut buffer, Theme::VirtualBoy);

        // Luma for white (255, 255, 255) is 254 (due to >> 8 approx)
        assert_eq!(buffer[0], 255);
        assert_eq!(buffer[1], 0); // G cleared
        assert_eq!(buffer[2], 0); // B cleared
        assert_eq!(buffer[3], 255); // Alpha preserved

        // Luma for black (0, 0, 0) is 0
        assert_eq!(buffer[4], 0);
        assert_eq!(buffer[5], 0); // G cleared
        assert_eq!(buffer[6], 0); // B cleared
        assert_eq!(buffer[7], 128); // Alpha preserved
    }
}
