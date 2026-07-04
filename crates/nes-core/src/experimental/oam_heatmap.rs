//! Experimental visual heatmap generator for OAM (Object Attribute Memory) sprites.
//!
//! This module tracks NES sprites' physical screen presence over time, creating a
//! visual heatmap. This is useful for seeing where entities tend to cluster, patrol,
//! or die most frequently during a game session.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;
#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSpatialQuery;

#[cfg(feature = "nova")]
/// Tracks OAM sprite positions to generate a spatial heatmap.
pub struct OamHeatmap {
    heat: std::vec::Vec<f32>,
    decay_rate: f32,
    intensity: f32,
}

#[cfg(feature = "nova")]
impl Default for OamHeatmap {
    fn default() -> Self {
        Self::new(0.95, 0.1)
    }
}

#[cfg(feature = "nova")]
impl OamHeatmap {
    /// Creates a new OAM heatmap tracker.
    ///
    /// * `decay_rate` - How quickly heat dissipates each frame (e.g. 0.95 = 5% decay).
    /// * `intensity` - How much heat is added per sprite region per frame.
    #[must_use]
    pub fn new(decay_rate: f32, intensity: f32) -> Self {
        Self {
            heat: vec![0.0; FRAME_WIDTH * FRAME_HEIGHT],
            decay_rate,
            intensity,
        }
    }

    /// Records the current frame's sprite positions into the heatmap.
    pub fn record_frame(&mut self, core: &NesCore) {
        let query = OamSpatialQuery::new(core);

        for sprite in query.sprites() {
            // Sprites positioned below 239 are generally off-screen or hidden.
            if sprite.y >= FRAME_HEIGHT as u8 {
                continue;
            }

            let start_x = sprite.x as usize;
            let start_y = sprite.y as usize;

            // Assume 8x8 sprite dimensions for simplicity and general overlap coverage
            for dy in 0..8 {
                for dx in 0..8 {
                    let x = start_x.saturating_add(dx);
                    let y = start_y.saturating_add(dy);

                    if x < FRAME_WIDTH && y < FRAME_HEIGHT {
                        let idx = y * FRAME_WIDTH + x;
                        self.heat[idx] = (self.heat[idx] + self.intensity).min(1.0);
                    }
                }
            }
        }
    }

    /// Decays the overall heat map. Should be called once per frame.
    pub fn decay_frame(&mut self) {
        for h in &mut self.heat {
            *h *= self.decay_rate;
            if *h < 0.001 {
                *h = 0.0;
            }
        }
    }

    /// Renders the current heatmap as a 256x240 BMP image matching the NES resolution.
    /// Hot regions are red, warm are yellow/green, cool are blue.
    pub fn render_bmp(&self) -> Result<std::vec::Vec<u8>, String> {
        let mut rgba = vec![0u8; FRAME_WIDTH * FRAME_HEIGHT * 4];

        for (idx, &heat) in self.heat.iter().enumerate() {
            let pixel_idx = idx * 4;

            // Simple thermal gradient mapping
            let (r, g, b) = if heat < 0.33 {
                // Cool: Black to Blue
                (0, 0, (heat * 3.0 * 255.0) as u8)
            } else if heat < 0.66 {
                // Warm: Blue to Green
                (
                    0,
                    ((heat - 0.33) * 3.0 * 255.0) as u8,
                    255 - ((heat - 0.33) * 3.0 * 255.0) as u8,
                )
            } else {
                // Hot: Green to Red
                (
                    ((heat - 0.66) * 3.0 * 255.0) as u8,
                    255 - ((heat - 0.66) * 3.0 * 255.0) as u8,
                    0,
                )
            };

            rgba[pixel_idx] = r;
            rgba[pixel_idx + 1] = g;
            rgba[pixel_idx + 2] = b;
            rgba[pixel_idx + 3] = 255;
        }

        encode_bmp(FRAME_WIDTH, FRAME_HEIGHT, &rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::Command;

    #[test]
    fn oam_heatmap_initialization() {
        let heatmap = OamHeatmap::new(0.85, 0.25);
        assert_eq!(heatmap.decay_rate, 0.85);
        assert_eq!(heatmap.intensity, 0.25);
        assert_eq!(heatmap.heat.len(), FRAME_WIDTH * FRAME_HEIGHT);
    }

    #[test]
    fn oam_heatmap_records_and_decays() {
        let mut core = NesCore::new();
        let mut heatmap = OamHeatmap::default();

        let _ = core.execute(Command::StepFrame);

        heatmap.record_frame(&core);
        heatmap.decay_frame();

        // Heatmap should render without error
        let bmp = heatmap.render_bmp().unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }

    #[test]
    fn oam_heatmap_covers_color_bands() {
        let mut heatmap = OamHeatmap::new(0.5, 0.1);

        // Manually inject heat levels to cover color bands
        heatmap.heat[0] = 0.1; // Cool
        heatmap.heat[1] = 0.5; // Warm
        heatmap.heat[2] = 0.9; // Hot

        // Render BMP to hit the color logic
        let bmp = heatmap.render_bmp().unwrap();
        assert_eq!(&bmp[0..2], b"BM");

        // Force decay to cover the 0.0 clamping
        for _ in 0..10 {
            heatmap.decay_frame();
        }
        assert_eq!(heatmap.heat[0], 0.0);
    }
}
