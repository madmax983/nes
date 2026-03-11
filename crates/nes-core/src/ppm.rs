//! Portable Pixmap (PPM) encoding utilities.

/// Encodes a raw RGBA pixel buffer into a P6 PPM file format.
///
/// Converts a 32-bit RGBA byte slice into a 24-bit RGB P6 binary PPM format,
/// stripping the alpha channel.
///
/// # Arguments
///
/// * `width` - The width of the image in pixels.
/// * `height` - The height of the image in pixels.
/// * `rgba` - The raw RGBA pixel data (length must be `width * height * 4`).
///
/// # Returns
///
/// A dynamically allocated `Vec<u8>` containing the complete PPM file bytes.
pub fn encode_ppm(width: usize, height: usize, rgba: &[u8]) -> Vec<u8> {
    let mut ppm = Vec::with_capacity(32 + width * height * 3);
    ppm.extend_from_slice(format!("P6\n{width} {height}\n255\n").as_bytes());
    for px in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&px[..3]);
    }
    ppm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_ppm_emits_expected_headers_and_pixel_layout() {
        let ppm = encode_ppm(2, 1, &[1, 2, 3, 255, 4, 5, 6, 255]);
        assert!(ppm.starts_with(b"P6\n2 1\n255\n"));
        assert!(ppm.ends_with(&[1, 2, 3, 4, 5, 6]));
    }
}
