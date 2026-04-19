//! Experimental visual heatmap generator for sprite occurrences.
//!
//! This module tracks where sprites appear on the screen over time and renders a thermal heatmap
//! image of the 256x240 screen space, useful for analyzing game entity distribution.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSpatialQuery;

#[cfg(feature = "nova")]
/// Tracks sprite positions to generate a visual heatmap.
pub struct SpriteHeatmap {
    heat: std::vec::Vec<f32>,
    decay_rate: f32,
    intensity: f32,
}

#[cfg(feature = "nova")]
impl Default for SpriteHeatmap {
    fn default() -> Self {
        Self::new(0.95, 0.1)
    }
}

#[cfg(feature = "nova")]
impl SpriteHeatmap {
    /// Creates a new heatmap tracker.
    ///
    /// * `decay_rate` - How quickly heat dissipates each frame (e.g. 0.95 = 5% decay).
    /// * `intensity` - How much heat is added per sprite pixel.
    #[must_use]
    pub fn new(decay_rate: f32, intensity: f32) -> Self {
        Self {
            heat: vec![0.0; 256 * 240],
            decay_rate,
            intensity,
        }
    }

    /// Records all sprite positions from the core's current state.
    pub fn record_frame(&mut self, core: &NesCore) {
        let oam = OamSpatialQuery::new(core);
        for sprite in oam.sprites() {
            // Sprites with y >= 240 are off-screen
            if sprite.y >= 240 {
                continue;
            }

            // Add heat to the 8x8 bounding box of each sprite
            for dy in 0..8 {
                for dx in 0..8 {
                    let y = (sprite.y as usize) + dy;
                    let x = (sprite.x as usize) + dx;

                    if x < 256 && y < 240 {
                        let idx = y * 256 + x;
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

    /// Renders the current heatmap as a 256x240 BMP image.
    /// Hot areas are red, warm are yellow, cool are blue.
    pub fn render_bmp(&self) -> Result<std::vec::Vec<u8>, String> {
        let width = 256;
        let height = 240;
        let mut rgba = vec![0u8; width * height * 4];

        for (addr, &heat) in self.heat.iter().enumerate() {
            let idx = addr * 4;

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

            rgba[idx] = r;
            rgba[idx + 1] = g;
            rgba[idx + 2] = b;
            rgba[idx + 3] = 255;
        }

        encode_bmp(width, height, &rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn should_initialize_sprite_heatmap_with_provided_parameters() {
        let heatmap = SpriteHeatmap::new(0.85, 0.25);

        assert_eq!(
            heatmap.decay_rate, 0.85,
            "Decay rate should match the provided value"
        );
        assert_eq!(
            heatmap.intensity, 0.25,
            "Intensity should match the provided value"
        );
        assert_eq!(
            heatmap.heat.len(),
            256 * 240,
            "Heat array should be exactly 256x240"
        );
        assert!(
            heatmap.heat.iter().all(|&h| h == 0.0),
            "All heat values should be initialized to 0.0"
        );
    }

    #[test]
    fn sprite_heatmap_records_and_decays() {
        let mut core = NesCore::new();
        // Force a sprite into OAM so we have something to record
        core.write_cpu_bus(0x2003, 0); // OAMADDR
        core.write_cpu_bus(0x2004, 100); // Y
        core.write_cpu_bus(0x2004, 0); // Tile
        core.write_cpu_bus(0x2004, 0); // Attr
        core.write_cpu_bus(0x2004, 100); // X

        let mut heatmap = SpriteHeatmap::default();
        heatmap.record_frame(&core);
        heatmap.decay_frame();

        // Heatmap should render without error
        let bmp = heatmap.render_bmp().unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }

    #[test]
    fn sprite_heatmap_covers_color_bands() {
        let mut heatmap = SpriteHeatmap::new(0.5, 0.1);

        // Manually inject heat levels to cover color bands
        heatmap.heat[0] = 0.1; // Cool
        heatmap.heat[1] = 0.5; // Warm
        heatmap.heat[2] = 0.9; // Hot

        // Render BMP to hit the color logic
        let bmp = heatmap.render_bmp().unwrap();
        assert_eq!(&bmp[0..2], b"BM");

        // Force decay to cover the 0.0 clamping
        heatmap.decay_frame();
        heatmap.decay_frame();
        heatmap.decay_frame();
        heatmap.decay_frame();
        heatmap.decay_frame();
        heatmap.decay_frame();
        heatmap.decay_frame();
        assert_eq!(heatmap.heat[0], 0.0);
    }
}
