//! Experimental ambient light analyzer for NES framebuffers.
//!
//! This module analyzes the NES RGBA framebuffer and calculates the average
//! color for different screen regions (top, bottom, left, right, center).
//! This data can be used to drive physical RGB LED strips behind a monitor
//! (Ambilight) or to dynamically theme the surrounding UI to match the game.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Represents an RGB color.
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// The average colors of different screen regions.
pub struct AmbientColors {
    pub top: Color,
    pub bottom: Color,
    pub left: Color,
    pub right: Color,
    pub center: Color,
}

#[cfg(feature = "nova")]
/// Analyzes framebuffers to extract ambient lighting colors.
pub struct AmbilightAnalyzer {
    // Thickness of the edge regions in pixels
    edge_thickness: usize,
}

#[cfg(feature = "nova")]
impl AmbilightAnalyzer {
    /// Creates a new `AmbilightAnalyzer` with the specified edge thickness.
    #[must_use]
    pub fn new(edge_thickness: usize) -> Self {
        Self { edge_thickness }
    }

    /// Analyzes the provided RGBA framebuffer and calculates the average
    /// color for the top, bottom, left, right, and center regions.
    ///
    /// The input buffer should be in `RGBA8` format (4 bytes per pixel)
    /// and match the standard NES frame dimensions (256x240).
    #[must_use]
    pub fn analyze(&self, framebuffer: &[u8]) -> AmbientColors {
        if framebuffer.len() < FRAME_WIDTH * FRAME_HEIGHT * 4 {
            return AmbientColors::default();
        }

        let mut top_r = 0u64;
        let mut top_g = 0u64;
        let mut top_b = 0u64;
        let mut top_count = 0u64;

        let mut bottom_r = 0u64;
        let mut bottom_g = 0u64;
        let mut bottom_b = 0u64;
        let mut bottom_count = 0u64;

        let mut left_r = 0u64;
        let mut left_g = 0u64;
        let mut left_b = 0u64;
        let mut left_count = 0u64;

        let mut right_r = 0u64;
        let mut right_g = 0u64;
        let mut right_b = 0u64;
        let mut right_count = 0u64;

        let mut center_r = 0u64;
        let mut center_g = 0u64;
        let mut center_b = 0u64;
        let mut center_count = 0u64;

        for y in 0..FRAME_HEIGHT {
            for x in 0..FRAME_WIDTH {
                let idx = (y * FRAME_WIDTH + x) * 4;
                let r = u64::from(framebuffer[idx]);
                let g = u64::from(framebuffer[idx + 1]);
                let b = u64::from(framebuffer[idx + 2]);

                if y < self.edge_thickness {
                    top_r += r;
                    top_g += g;
                    top_b += b;
                    top_count += 1;
                } else if y >= FRAME_HEIGHT - self.edge_thickness {
                    bottom_r += r;
                    bottom_g += g;
                    bottom_b += b;
                    bottom_count += 1;
                } else if x < self.edge_thickness {
                    left_r += r;
                    left_g += g;
                    left_b += b;
                    left_count += 1;
                } else if x >= FRAME_WIDTH - self.edge_thickness {
                    right_r += r;
                    right_g += g;
                    right_b += b;
                    right_count += 1;
                } else {
                    center_r += r;
                    center_g += g;
                    center_b += b;
                    center_count += 1;
                }
            }
        }

        let avg = |sum: u64, count: u64| -> u8 { if count == 0 { 0 } else { (sum / count) as u8 } };

        AmbientColors {
            top: Color {
                r: avg(top_r, top_count),
                g: avg(top_g, top_count),
                b: avg(top_b, top_count),
            },
            bottom: Color {
                r: avg(bottom_r, bottom_count),
                g: avg(bottom_g, bottom_count),
                b: avg(bottom_b, bottom_count),
            },
            left: Color {
                r: avg(left_r, left_count),
                g: avg(left_g, left_count),
                b: avg(left_b, left_count),
            },
            right: Color {
                r: avg(right_r, right_count),
                g: avg(right_g, right_count),
                b: avg(right_b, right_count),
            },
            center: Color {
                r: avg(center_r, center_count),
                g: avg(center_g, center_count),
                b: avg(center_b, center_count),
            },
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::constants::FRAME_RGBA_BYTES;

    #[test]
    fn test_analyze_pure_red_frame() {
        let analyzer = AmbilightAnalyzer::new(10);
        let mut frame = vec![0; FRAME_RGBA_BYTES];
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = 255; // R
            pixel[1] = 0; // G
            pixel[2] = 0; // B
            pixel[3] = 255; // A
        }

        let colors = analyzer.analyze(&frame);
        let expected = Color { r: 255, g: 0, b: 0 };

        assert_eq!(colors.top, expected);
        assert_eq!(colors.bottom, expected);
        assert_eq!(colors.left, expected);
        assert_eq!(colors.right, expected);
        assert_eq!(colors.center, expected);
    }

    #[test]
    fn test_analyze_distinct_regions() {
        let edge_thickness = 10;
        let analyzer = AmbilightAnalyzer::new(edge_thickness);
        let mut frame = vec![0; FRAME_RGBA_BYTES];

        for y in 0..FRAME_HEIGHT {
            for x in 0..FRAME_WIDTH {
                let idx = (y * FRAME_WIDTH + x) * 4;
                if y < edge_thickness {
                    // Top: Green
                    frame[idx + 1] = 255;
                } else if y >= FRAME_HEIGHT - edge_thickness {
                    // Bottom: Blue
                    frame[idx + 2] = 255;
                } else if x < edge_thickness {
                    // Left: Red
                    frame[idx] = 255;
                } else if x >= FRAME_WIDTH - edge_thickness {
                    // Right: White
                    frame[idx] = 255;
                    frame[idx + 1] = 255;
                    frame[idx + 2] = 255;
                } else {
                    // Center: Purple
                    frame[idx] = 128;
                    frame[idx + 2] = 128;
                }
            }
        }

        let colors = analyzer.analyze(&frame);

        assert_eq!(colors.top, Color { r: 0, g: 255, b: 0 });
        assert_eq!(colors.bottom, Color { r: 0, g: 0, b: 255 });
        assert_eq!(colors.left, Color { r: 255, g: 0, b: 0 });
        assert_eq!(
            colors.right,
            Color {
                r: 255,
                g: 255,
                b: 255
            }
        );
        assert_eq!(
            colors.center,
            Color {
                r: 128,
                g: 0,
                b: 128
            }
        );
    }

    #[test]
    fn test_zero_thickness() {
        let analyzer = AmbilightAnalyzer::new(0);
        let mut frame = vec![0; FRAME_RGBA_BYTES];
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = 255;
            pixel[1] = 255;
            pixel[2] = 255;
            pixel[3] = 255;
        }

        let colors = analyzer.analyze(&frame);

        // Everything should be 0 except center
        let black = Color { r: 0, g: 0, b: 0 };
        assert_eq!(colors.top, black);
        assert_eq!(colors.bottom, black);
        assert_eq!(colors.left, black);
        assert_eq!(colors.right, black);
        assert_eq!(
            colors.center,
            Color {
                r: 255,
                g: 255,
                b: 255
            }
        );
    }
}
