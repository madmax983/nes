//! Uncompressed BMP image encoding.
//!
//! This module provides a minimal, allocation-efficient encoder for transforming
//! raw RGBA framebuffer slices into standard 24-bit uncompressed Windows BMP
//! byte vectors. It is primarily used for generating emulator screenshots.

/// Encodes RGB(A) pixel data into a BMP image byte vector.
///
/// The BMP format stores pixels "bottom-up" in `BGR` order. This function
/// strips the alpha channel from the input `rgba` slice and handles the
/// necessary 4-byte row padding required by the BMP specification.
///
/// # Parameters
///
/// * `width` - The image width in pixels.
/// * `height` - The image height in pixels.
/// * `rgba` - The raw pixel data in `RGBA8` format. The slice length must be exactly `width * height * 4`.
///
/// # Errors
///
/// Returns a descriptive string error if the dimensions are too large and cause
/// internal size calculations to overflow `usize` or `u32` bounds.
///
/// # Examples
///
/// ```
/// use nes_core::bmp::encode_bmp;
///
/// // A 2x2 image containing 4 pixels in RGBA format (Red, Green, Blue, White).
/// let rgba = vec![
///     255, 0, 0, 255,   0, 255, 0, 255,
///     0, 0, 255, 255,   255, 255, 255, 255,
/// ];
///
/// let bmp_bytes = encode_bmp(2, 2, &rgba).unwrap();
/// assert_eq!(&bmp_bytes[0..2], b"BM");
/// ```
pub fn encode_bmp(width: usize, height: usize, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let row_bytes = width
        .checked_mul(3)
        .ok_or_else(|| "bmp row size overflow".to_owned())?;
    let row_padding = (4 - (row_bytes % 4)) % 4;
    let stride = row_bytes
        .checked_add(row_padding)
        .ok_or_else(|| "bmp stride overflow".to_owned())?;
    let pixel_data_size = stride
        .checked_mul(height)
        .ok_or_else(|| "bmp pixel data size overflow".to_owned())?;
    let file_size = 54usize
        .checked_add(pixel_data_size)
        .ok_or_else(|| "bmp file size overflow".to_owned())?;

    let width_i32 = i32::try_from(width).map_err(|_| "bmp width out of range".to_owned())?;
    let height_i32 = i32::try_from(height).map_err(|_| "bmp height out of range".to_owned())?;
    let file_size_u32 =
        u32::try_from(file_size).map_err(|_| "bmp file size out of range".to_owned())?;
    let pixel_data_size_u32 = u32::try_from(pixel_data_size)
        .map_err(|_| "bmp pixel data size out of range".to_owned())?;

    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&file_size_u32.to_le_bytes());
    bmp.extend_from_slice(&0u16.to_le_bytes());
    bmp.extend_from_slice(&0u16.to_le_bytes());
    bmp.extend_from_slice(&54u32.to_le_bytes());

    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&width_i32.to_le_bytes());
    bmp.extend_from_slice(&height_i32.to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&pixel_data_size_u32.to_le_bytes());
    bmp.extend_from_slice(&2_835u32.to_le_bytes());
    bmp.extend_from_slice(&2_835u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());

    for y in (0..height).rev() {
        let row_start = y * width * 4;
        for x in 0..width {
            let idx = row_start + x * 4;
            bmp.push(rgba[idx + 2]);
            bmp.push(rgba[idx + 1]);
            bmp.push(rgba[idx]);
        }
        bmp.extend(std::iter::repeat_n(0, row_padding));
    }

    Ok(bmp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_bmp_multiplies_row_and_column_indices_for_bottom_up_bgr_layout() {
        let rgba = vec![
            // Top row.
            255, 0, 0, 255, 0, 255, 0, 255, // Bottom row.
            0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let bmp = encode_bmp(2, 2, &rgba).expect("bmp encoding should succeed");
        assert_eq!(bmp.len(), 70);

        // BMP stores rows bottom-up and colors as B,G,R with row padding.
        assert_eq!(&bmp[54..62], &[255, 0, 0, 255, 255, 255, 0, 0]);
        assert_eq!(&bmp[62..70], &[0, 0, 255, 0, 255, 0, 0, 0]);
    }

    #[test]
    fn encode_bmp_produces_expected_headers_and_pixel_order() {
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, // top row: red, green
            0, 0, 255, 255, 255, 255, 255, 255, // bottom row: blue, white
        ];
        let bmp = encode_bmp(2, 2, &rgba).expect("encode bmp");
        assert_eq!(&bmp[0..2], b"BM");
        assert_eq!(bmp.len(), 70);
        assert_eq!(u32::from_le_bytes([bmp[2], bmp[3], bmp[4], bmp[5]]), 70);
        assert_eq!(u32::from_le_bytes([bmp[10], bmp[11], bmp[12], bmp[13]]), 54);
        assert_eq!(u32::from_le_bytes([bmp[34], bmp[35], bmp[36], bmp[37]]), 16);
        assert_eq!(
            &bmp[54..],
            &[
                255, 0, 0, 255, 255, 255, 0, 0, // bottom row (BGR + padding)
                0, 0, 255, 0, 255, 0, 0, 0, // top row (BGR + padding)
            ]
        );
    }

    #[test]
    fn encode_bmp_uses_expected_padding_for_odd_row_widths() {
        let rgba = vec![12, 34, 56, 255, 90, 80, 70, 255];
        let bmp = encode_bmp(1, 2, &rgba).expect("encode bmp with row padding");
        assert_eq!(bmp.len(), 62);
        assert_eq!(u32::from_le_bytes([bmp[34], bmp[35], bmp[36], bmp[37]]), 8);
        assert_eq!(
            &bmp[54..],
            &[
                70, 80, 90, 0, // bottom row + 1 byte padding
                56, 34, 12, 0, // top row + 1 byte padding
            ]
        );
    }

    #[test]
    fn encode_bmp_uses_expected_padding_for_width_3() {
        let rgba = vec![
            10, 20, 30, 255, 40, 50, 60, 255, 70, 80, 90, 255, // top row
        ];
        let bmp = encode_bmp(3, 1, &rgba).expect("encode bmp with 3-byte padding");
        // file_size = 54 + (3 * 3 + 3) * 1 = 54 + 12 = 66
        assert_eq!(bmp.len(), 66);
        assert_eq!(u32::from_le_bytes([bmp[34], bmp[35], bmp[36], bmp[37]]), 12);
        assert_eq!(
            &bmp[54..],
            &[
                30, 20, 10, 60, 50, 40, 90, 80, 70, 0, 0, 0, // row + 3 bytes padding
            ]
        );
    }
}
