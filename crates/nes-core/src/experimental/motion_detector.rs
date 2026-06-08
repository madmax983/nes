//! Experimental tools for detecting motion in the NES framebuffer.

#[cfg(feature = "nova")]
use crate::constants::FRAME_RGBA_BYTES;

#[cfg(feature = "nova")]
pub struct MotionDetector {
    threshold: u8,
    previous_frame: Vec<u8>,
}

#[cfg(feature = "nova")]
impl MotionDetector {
    pub fn new(threshold: u8) -> Self {
        Self {
            threshold,
            previous_frame: vec![0; FRAME_RGBA_BYTES],
        }
    }

    pub fn isolate_motion(&mut self, current_frame: &mut [u8]) {
        if current_frame.len() != FRAME_RGBA_BYTES {
            return;
        }

        for (curr, prev) in current_frame
            .chunks_exact_mut(4)
            .zip(self.previous_frame.chunks_exact_mut(4))
        {
            let r_diff = curr[0].abs_diff(prev[0]);
            let g_diff = curr[1].abs_diff(prev[1]);
            let b_diff = curr[2].abs_diff(prev[2]);

            // Save the current pixel state to previous before modifying it
            prev[0] = curr[0];
            prev[1] = curr[1];
            prev[2] = curr[2];
            prev[3] = curr[3];

            if r_diff < self.threshold && g_diff < self.threshold && b_diff < self.threshold {
                curr[0] = 0;
                curr[1] = 0;
                curr[2] = 0;
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn isolate_motion_blanks_static_pixels_and_keeps_moving_ones() {
        let mut detector = MotionDetector::new(10);

        let mut first_frame = vec![100; FRAME_RGBA_BYTES];
        first_frame[3] = 255; // Alpha

        // Push the first frame to become the previous_frame
        detector.isolate_motion(&mut first_frame);

        let mut second_frame = vec![100; FRAME_RGBA_BYTES];
        second_frame[3] = 255; // Alpha

        // Pixel 0 is static, Pixel 1 is moving (above threshold)
        second_frame[4] = 120; // R
        second_frame[5] = 100; // G
        second_frame[6] = 100; // B
        second_frame[7] = 255; // Alpha

        detector.isolate_motion(&mut second_frame);

        // Pixel 0 should be blanked out
        assert_eq!(second_frame[0], 0);
        assert_eq!(second_frame[1], 0);
        assert_eq!(second_frame[2], 0);
        assert_eq!(second_frame[3], 255); // Alpha preserved

        // Pixel 1 should be kept
        assert_eq!(second_frame[4], 120);
        assert_eq!(second_frame[5], 100);
        assert_eq!(second_frame[6], 100);
        assert_eq!(second_frame[7], 255); // Alpha preserved
    }
}
