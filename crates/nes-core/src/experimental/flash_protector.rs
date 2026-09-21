#[cfg(feature = "nova")]
pub struct FlashProtector {
    previous_luminance: u8,
    cooldown_frames: u8,
}

#[cfg(feature = "nova")]
impl Default for FlashProtector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "nova")]
impl FlashProtector {
    pub fn new() -> Self {
        Self {
            previous_luminance: 0,
            cooldown_frames: 0,
        }
    }

    pub fn process_frame(&mut self, frame: &mut [u8]) {
        // Calculate average luminance
        let mut total_luminance: u64 = 0;
        let pixel_count = (frame.len() / 4) as u64;

        if pixel_count == 0 {
            return;
        }

        for chunk in frame.chunks_exact(4) {
            // Rec. 601 Luma approximation: 0.299 R + 0.587 G + 0.114 B
            let r = chunk[0] as u64;
            let g = chunk[1] as u64;
            let b = chunk[2] as u64;
            let lum = (r * 299 + g * 587 + b * 114) / 1000;
            total_luminance += lum;
        }

        let avg_luminance = (total_luminance / pixel_count) as u8;

        // Detect flash (large positive jump in luminance)
        let diff = avg_luminance.saturating_sub(self.previous_luminance);

        if diff > 100 {
            self.cooldown_frames = 3;
        }

        if self.cooldown_frames > 0 {
            // Dampen the frame
            for (i, pixel) in frame.iter_mut().enumerate() {
                // don't dampen alpha
                if i % 4 != 3 {
                    *pixel = pixel.saturating_sub(diff / 2);
                }
            }
            self.cooldown_frames -= 1;
        }

        self.previous_luminance = avg_luminance;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flash_protector_dampens_flash() {
        let mut protector = FlashProtector::new();
        let mut dark_frame = vec![10; 256 * 240 * 4];
        protector.process_frame(&mut dark_frame);

        let mut bright_frame = vec![255; 256 * 240 * 4];
        for i in (3..bright_frame.len()).step_by(4) {
            bright_frame[i] = 255;
        }
        protector.process_frame(&mut bright_frame);

        assert!(bright_frame[0] < 255);
    }
}
