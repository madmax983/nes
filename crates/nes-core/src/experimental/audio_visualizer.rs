use crate::bmp::encode_bmp;

/// Renders a waveform of the given f32 audio samples into a BMP byte vector.
pub fn render_waveform_bmp(samples: &[f32], width: usize, height: usize) -> Result<Vec<u8>, String> {
    if samples.is_empty() || width == 0 || height == 0 {
        return Err("Invalid dimensions or empty samples".to_string());
    }

    let mut rgba = vec![0u8; width * height * 4];
    let half_height = height as f32 / 2.0;

    // Draw center line
    let center_y = height / 2;
    for x in 0..width {
        let idx = (center_y * width + x) * 4;
        rgba[idx] = 50;     // R
        rgba[idx + 1] = 50; // G
        rgba[idx + 2] = 50; // B
        rgba[idx + 3] = 255;// A
    }

    // Determine how many samples per pixel width
    let samples_per_pixel = samples.len() as f32 / width as f32;

    for x in 0..width {
        let start_idx = (x as f32 * samples_per_pixel) as usize;
        let mut end_idx = ((x + 1) as f32 * samples_per_pixel) as usize;
        if end_idx > samples.len() {
            end_idx = samples.len();
        }
        if start_idx >= end_idx {
            continue;
        }

        // Find min and max sample in this bucket
        let mut min_val = f32::MAX;
        let mut max_val = f32::MIN;
        for &sample in &samples[start_idx..end_idx] {
            if sample < min_val { min_val = sample; }
            if sample > max_val { max_val = sample; }
        }

        // Clamp to [-1.0, 1.0]
        min_val = min_val.clamp(-1.0, 1.0);
        max_val = max_val.clamp(-1.0, 1.0);

        // Map to Y coordinates
        let y_min = (half_height - (max_val * half_height)) as usize;
        let y_max = (half_height - (min_val * half_height)) as usize;

        let y_start = y_min.min(height - 1);
        let y_end = y_max.min(height - 1);

        for y in y_start..=y_end {
            let idx = (y * width + x) * 4;
            rgba[idx] = 0;      // R
            rgba[idx + 1] = 255;// G
            rgba[idx + 2] = 0;  // B
            rgba[idx + 3] = 255;// A
        }
    }

    encode_bmp(width, height, &rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_waveform_bmp_basic() {
        // Create a sine wave
        let width = 100;
        let height = 50;
        let samples: Vec<f32> = (0..width)
            .map(|i| (i as f32 * 0.1).sin())
            .collect();

        let result = render_waveform_bmp(&samples, width, height);
        assert!(result.is_ok(), "Expected render_waveform_bmp to return Ok");
        let bmp_data = result.unwrap();

        // Basic BMP validation
        assert_eq!(&bmp_data[0..2], b"BM", "Must be a valid BMP file");
        assert!(bmp_data.len() > 54, "BMP must include header and pixel data");
    }
}
