//! Experimental visual heatmap generator for sprite screen presence.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSpatialQuery;

#[cfg(feature = "nova")]
/// Tracks sprite screen presence to generate a visual heatmap.
pub struct SpriteHeatmap {
    pub heat: std::vec::Vec<f32>,
    pub decay_rate: f32,
    pub intensity: f32,
}

#[cfg(feature = "nova")]
impl Default for SpriteHeatmap {
    fn default() -> Self {
        Self::new(0.95, 0.1)
    }
}

#[cfg(feature = "nova")]
impl SpriteHeatmap {
    #[must_use]
    pub fn new(decay_rate: f32, intensity: f32) -> Self {
        Self {
            heat: vec![0.0; 256 * 240],
            decay_rate,
            intensity,
        }
    }

    pub fn record_frame(&mut self, core: &NesCore) {
        let query = OamSpatialQuery::new(core);
        for sprite in query.sprites() {
            // Check if the sprite is actually somewhat active or valid.
            // A common pattern is Y >= 239 means offscreen.
            if sprite.y >= 239 {
                continue;
            }

            let start_x = sprite.x as usize;
            let start_y = sprite.y as usize;

            // Sprites are 8x8.
            for dy in 0..8 {
                for dx in 0..8 {
                    let px = start_x + dx;
                    let py = start_y + dy;

                    if px < 256 && py < 240 {
                        let idx = py * 256 + px;
                        self.heat[idx] = (self.heat[idx] + self.intensity).min(1.0);
                    }
                }
            }
        }
    }

    pub fn decay_frame(&mut self) {
        for h in &mut self.heat {
            *h *= self.decay_rate;
            if *h < 0.001 {
                *h = 0.0;
            }
        }
    }

    pub fn render_bmp(&self) -> Result<std::vec::Vec<u8>, String> {
        let width = 256;
        let height = 240;
        let mut rgba = vec![0u8; width * height * 4];

        for y in 0..height {
            for x in 0..width {
                let heat_val = self.heat[y * width + x];
                let idx = (y * width + x) * 4;

                let (r, g, b) = if heat_val < 0.33 {
                    // Cool: Black to Blue
                    (0, 0, (heat_val * 3.0 * 255.0) as u8)
                } else if heat_val < 0.66 {
                    // Warm: Blue to Green
                    (
                        0,
                        ((heat_val - 0.33) * 3.0 * 255.0) as u8,
                        255 - ((heat_val - 0.33) * 3.0 * 255.0) as u8,
                    )
                } else {
                    // Hot: Green to Red
                    (
                        ((heat_val - 0.66) * 3.0 * 255.0) as u8,
                        255 - ((heat_val - 0.66) * 3.0 * 255.0) as u8,
                        0,
                    )
                };

                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 255;
            }
        }

        encode_bmp(width, height, &rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn should_initialize_sprite_heatmap_with_provided_parameters() {
        let heatmap = SpriteHeatmap::new(0.85, 0.25);
        assert_eq!(heatmap.decay_rate, 0.85);
        assert_eq!(heatmap.intensity, 0.25);
        assert_eq!(heatmap.heat.len(), 256 * 240);
        assert!(heatmap.heat.iter().all(|&h| h == 0.0));
    }

    #[test]
    fn sprite_heatmap_records_and_decays() {
        let mut core = NesCore::new();
        let mut dummy_page = [0xff; 256]; // Most will have Y = 255 (offscreen)
        dummy_page[0] = 10; // Sprite 0 Y
        dummy_page[3] = 10; // Sprite 0 X
        core.load_cpu_bytes(0x0200, &dummy_page);
        core.write_cpu_bus(0x4014, 0x02); // OAM DMA

        for _ in 0..520 {
            let _ = core.execute(crate::Command::StepCpu);
        }

        let mut heatmap = SpriteHeatmap::new(0.95, 0.1);
        heatmap.record_frame(&core);

        let idx = 10 * 256 + 10;
        assert!(heatmap.heat[idx] > 0.0);

        heatmap.decay_frame();
        assert!(heatmap.heat[idx] < 0.1);

        let bmp = heatmap.render_bmp().unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }

    #[test]
    fn sprite_heatmap_covers_color_bands() {
        let mut heatmap = SpriteHeatmap::new(0.5, 0.1);
        heatmap.heat[0] = 0.1; // Cool
        heatmap.heat[1] = 0.5; // Warm
        heatmap.heat[2] = 0.9; // Hot

        let bmp = heatmap.render_bmp().unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }
}
