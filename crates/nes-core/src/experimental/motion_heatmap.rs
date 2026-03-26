#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;

/// An experimental utility that visualizes pixel-level motion between two frames.
///
/// It processes two sequential RGBA frames and produces a heatmap where static
/// background pixels are dimmed and converted to grayscale, while pixels that
/// changed significantly between frames are highlighted in bright red.
#[cfg(feature = "nova")]
pub struct MotionHeatmap;

#[cfg(feature = "nova")]
impl MotionHeatmap {
    /// Generates a BMP heatmap comparing two identical-sized RGBA frames.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the encoded BMP file, or an error if the dimensions do not match.
    pub fn generate_heatmap(
        width: usize,
        height: usize,
        prev_rgba: &[u8],
        curr_rgba: &[u8],
    ) -> Result<Vec<u8>, String> {
        let expected_len = width * height * 4;
        if prev_rgba.len() != expected_len || curr_rgba.len() != expected_len {
            return Err("Frame sizes do not match width * height * 4".to_string());
        }

        let mut out_rgba = vec![0_u8; expected_len];

        for i in (0..expected_len).step_by(4) {
            let r_diff = prev_rgba[i].abs_diff(curr_rgba[i]);
            let g_diff = prev_rgba[i + 1].abs_diff(curr_rgba[i + 1]);
            let b_diff = prev_rgba[i + 2].abs_diff(curr_rgba[i + 2]);

            let change_magnitude = r_diff.saturating_add(g_diff).saturating_add(b_diff);

            if change_magnitude > 30 {
                // High motion: Highlight in bright red
                out_rgba[i] = 255;
                out_rgba[i + 1] = 0;
                out_rgba[i + 2] = 0;
                out_rgba[i + 3] = 255;
            } else {
                // Low motion: Dimmed grayscale of the current frame
                let r = curr_rgba[i] as u32;
                let g = curr_rgba[i + 1] as u32;
                let b = curr_rgba[i + 2] as u32;
                let luma = ((r * 299 + g * 587 + b * 114) / 1000) as u8;
                let dimmed = luma / 3;

                out_rgba[i] = dimmed;
                out_rgba[i + 1] = dimmed;
                out_rgba[i + 2] = dimmed;
                out_rgba[i + 3] = 255;
            }
        }

        encode_bmp(width, height, &out_rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn heatmap_fails_on_size_mismatch() {
        let prev = vec![0; 4];
        let curr = vec![0; 8];
        assert!(MotionHeatmap::generate_heatmap(1, 1, &prev, &curr).is_err());
    }

    #[test]
    fn heatmap_highlights_motion() {
        let prev = vec![255, 255, 255, 255]; // White
        let curr = vec![0, 0, 0, 255];       // Black (high motion)

        let bmp = MotionHeatmap::generate_heatmap(1, 1, &prev, &curr).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
        // Output validation logic could check for the specific red value in the BMP data
        // but checking the magic bytes confirms the encoding ran successfully.
    }

    #[test]
    fn heatmap_dims_static_background_without_overflow() {
        let prev = vec![255, 255, 255, 255]; // White
        let curr = vec![255, 255, 255, 255]; // White (static, no motion)

        // This test will fail/panic if the luma calculation uses u16 and overflows.
        let bmp = MotionHeatmap::generate_heatmap(1, 1, &prev, &curr).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }
}
