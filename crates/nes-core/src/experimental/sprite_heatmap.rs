//! Experimental visual heatmap generator for OAM sprites.
//!
//! This module tracks where sprites are drawn over time and renders a thermal heatmap
//! image of the 256x240 screen, useful for profiling AI movement or hotspot analysis.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSpatialQuery;

#[cfg(feature = "nova")]
/// Tracks OAM sprite positions to generate a visual heatmap.
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
    /// Creates a new sprite heatmap tracker.
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

    /// Records all sprite bounding boxes from the core's OAM.
    pub fn record_frame(&mut self, core: &NesCore) {
        let query = OamSpatialQuery::new(core);
        for sprite in query.sprites() {
            // Ignore sprites parked offscreen (usually Y >= 239)
            if sprite.y >= 239 {
                continue;
            }

            // Apply heat to the 8x8 area of the sprite
            for dy in 0..8 {
                for dx in 0..8 {
                    let px = sprite.x.saturating_add(dx) as usize;
                    let py = sprite.y.saturating_add(dy) as usize;

                    if px < 256 && py < 240 {
                        let idx = py * 256 + px;
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
    /// Hot pixels are red, warm are yellow, cool are blue.
    pub fn render_bmp(&self) -> Result<std::vec::Vec<u8>, String> {
        let width = 256;
        let height = 240;
        let mut rgba = vec![0u8; width * height * 4];

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

        encode_bmp(width, height, &rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::Command;
    use crate::NesCore;

    #[test]
    fn sprite_heatmap_records_and_decays() {
        let mut core = NesCore::new();
        // Load sprite data at 0x0200
        let mut dummy_page = [0xff; 256];
        dummy_page[0] = 10; // Y
        dummy_page[3] = 10; // X
        core.load_cpu_bytes(0x0200, &dummy_page);

        // Trigger OAM DMA from page 0x02
        core.write_cpu_bus(0x4014, 0x02);
        for _ in 0..180 {
            let _ = core.execute(Command::StepCpu);
        }

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
