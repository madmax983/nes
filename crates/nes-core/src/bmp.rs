/// Encodes RGB(A) pixel data into a BMP image byte vector.
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
}
