//! PPM image encoding utilities.
//!
//! This module provides an allocation-efficient encoder for transforming
//! raw RGBA framebuffer slices into standard P6 format PPM byte vectors.

use std::io::{self, Write};

/// Encodes RGB(A) pixel data into a PPM image byte vector.
///
/// The PPM format is a simple text-header based format (P6) followed by raw RGB bytes.
/// This function strips the alpha channel from the input `rgba` slice and constructs
/// the appropriate header.
///
/// # Parameters
///
/// * `width` - The image width in pixels.
/// * `height` - The image height in pixels.
/// * `rgba` - The raw pixel data in `RGBA8` format. The slice length must be exactly `width * height * 4`.
///
/// # Errors
///
/// Returns a descriptive IO error if the dimensions are too large and cause
/// internal size calculations to overflow bounds, or if the provided `rgba` buffer
/// length does not match the expected dimensions.
///
/// # Examples
///
/// ```
/// use nes_core::ppm::encode_ppm;
///
/// // A 2x1 image containing 2 pixels in RGBA format (Red, Green).
/// let rgba = vec![
///     255, 0, 0, 255,   0, 255, 0, 255,
/// ];
///
/// let ppm_bytes = encode_ppm(2, 1, &rgba).unwrap();
/// assert!(ppm_bytes.starts_with(b"P6\n2 1\n255\n"));
/// ```
pub fn encode_ppm(width: usize, height: usize, rgba: &[u8]) -> io::Result<Vec<u8>> {
    let expected_len = width
        .checked_mul(height)
        .and_then(|px| px.checked_mul(4))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "frame size overflow"))?;

    if rgba.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "rgba buffer length does not match dimensions",
        ));
    }

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

    #[test]
    fn encode_ppm_returns_error_on_buffer_length_mismatch() {
        let err = encode_ppm(2, 1, &[1, 2, 3, 255]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn encode_ppm_returns_error_on_size_overflow() {
        let err = encode_ppm(usize::MAX, usize::MAX, &[]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(err.to_string(), "frame size overflow");
    }
}
