//! Uncompressed PPM image encoding.
//!
//! This module provides a minimal, allocation-efficient encoder for transforming
//! raw RGBA framebuffer slices into standard uncompressed PPM byte vectors.

/// Encodes RGB(A) pixel data into a PPM image byte vector.
///
/// # Parameters
///
/// * `width` - The image width in pixels.
/// * `height` - The image height in pixels.
/// * `rgba` - The raw pixel data in `RGBA8` format. The slice length must be exactly `width * height * 4`.
///
/// # Errors
///
/// Returns a descriptive error if the dimensions are invalid or the buffer length mismatches.
pub fn encode_ppm(width: usize, height: usize, rgba: &[u8]) -> std::io::Result<Vec<u8>> {
    let expected_len = width
        .checked_mul(height)
        .and_then(|w_h| w_h.checked_mul(4))
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "ppm dimensions overflow")
        })?;

    if rgba.len() != expected_len {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "rgba buffer length does not match width * height * 4",
        ));
    }

    use std::io::Write;
    let mut ppm = Vec::with_capacity(32 + width * height * 3);
    write!(&mut ppm, "P6\n{width} {height}\n255\n")?;
    for px in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&px[..3]);
    }
    Ok(ppm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_ppm_emits_expected_headers_and_pixel_layout() {
        let ppm = encode_ppm(2, 1, &[1, 2, 3, 255, 4, 5, 6, 255]).unwrap();
        assert!(ppm.starts_with(b"P6\n2 1\n255\n"));
        assert!(ppm.ends_with(&[1, 2, 3, 4, 5, 6]));
    }
}
