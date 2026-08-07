//! Experimental motion detection based on framebuffer analysis.
//!
//! Compares consecutive frames to detect areas of high visual change (motion).
//! Can be used to build bounding boxes around moving entities without reading OAM,
//! or as a trigger for automated gameplay recording.

use crate::{FRAME_WIDTH, FRAME_HEIGHT};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundingBox {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub struct MotionDetector {
    previous_frame: Vec<u8>,
    threshold: u8,
}

impl Default for MotionDetector {
    fn default() -> Self {
        Self::new(30)
    }
}

impl MotionDetector {
    pub fn new(threshold: u8) -> Self {
        Self {
            previous_frame: vec![0; FRAME_WIDTH * FRAME_HEIGHT * 4],
            threshold,
        }
    }

    pub fn detect_motion(&mut self, current_frame: &[u8]) -> Option<BoundingBox> {
        if current_frame.len() != self.previous_frame.len() {
            return None;
        }

        let mut min_x = FRAME_WIDTH as u32;
        let mut min_y = FRAME_HEIGHT as u32;
        let mut max_x = 0;
        let mut max_y = 0;
        let mut motion_found = false;

        for y in 0..FRAME_HEIGHT {
            for x in 0..FRAME_WIDTH {
                let idx = (y * FRAME_WIDTH + x) * 4;

                let r_diff = current_frame[idx].abs_diff(self.previous_frame[idx]);
                let g_diff = current_frame[idx + 1].abs_diff(self.previous_frame[idx + 1]);
                let b_diff = current_frame[idx + 2].abs_diff(self.previous_frame[idx + 2]);

                if r_diff > self.threshold || g_diff > self.threshold || b_diff > self.threshold {
                    min_x = min_x.min(x as u32);
                    min_y = min_y.min(y as u32);
                    max_x = max_x.max(x as u32);
                    max_y = max_y.max(y as u32);
                    motion_found = true;
                }
            }
        }

        self.previous_frame.copy_from_slice(current_frame);

        if motion_found {
            Some(BoundingBox {
                x: min_x,
                y: min_y,
                width: max_x - min_x + 1,
                height: max_y - min_y + 1,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_motion_detector_default() {
        let detector = MotionDetector::default();
        assert_eq!(detector.threshold, 30);
        assert_eq!(detector.previous_frame.len(), FRAME_WIDTH * FRAME_HEIGHT * 4);
    }

    #[test]
    fn test_motion_detector_invalid_len() {
        let mut detector = MotionDetector::new(50);
        let frame1 = vec![0; 10]; // wrong length
        assert!(detector.detect_motion(&frame1).is_none());
    }

    #[test]
    fn test_motion_detector_no_motion() {
        let mut detector = MotionDetector::new(50);
        let frame1 = vec![0; FRAME_WIDTH * FRAME_HEIGHT * 4];
        assert!(detector.detect_motion(&frame1).is_none());
    }

    #[test]
    fn test_motion_detector_motion() {
        let mut detector = MotionDetector::new(50);
        let mut frame2 = vec![0; FRAME_WIDTH * FRAME_HEIGHT * 4];

        // Draw a white pixel at (10, 10) in frame 2
        let idx = (10 * FRAME_WIDTH + 10) * 4;
        frame2[idx] = 255;
        frame2[idx + 1] = 255;
        frame2[idx + 2] = 255;
        frame2[idx + 3] = 255;

        // First pass sets the baseline (frame is all 0 initially, so this finds motion since current is white at 10,10)
        // Wait, detector starts with previous_frame all 0s.
        // So passing frame2 with a white pixel should detect motion immediately.
        let motion2 = detector.detect_motion(&frame2);
        assert!(motion2.is_some());
        let bb = motion2.unwrap();
        assert_eq!(bb.x, 10);
        assert_eq!(bb.y, 10);
        assert_eq!(bb.width, 1);
        assert_eq!(bb.height, 1);
    }
}
