//! Experimental visual heatmap generator for OAM sprites on screen.
//!
//! Tracks sprite coordinates over time and generates a 256x240 thermal heatmap.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSpatialQuery;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;

#[cfg(feature = "nova")]
pub struct SpriteHeatmap {
    heat: Vec<f32>,
    decay_rate: f32,
    intensity: f32,
}

#[cfg(feature = "nova")]
impl SpriteHeatmap {
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
            // Sprites hidden below Y=239 (e.g. 240-255) are generally off-screen
            if sprite.y >= 240 {
                continue;
            }

            for dy in 0..8 {
                for dx in 0..8 {
                    let y = sprite.y.saturating_add(dy) as usize;
                    let x = sprite.x.saturating_add(dx) as usize;

                    if y < 240 && x < 256 {
                        let idx = y * 256 + x;
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

    pub fn render_bmp(&self) -> Result<Vec<u8>, String> {
        let width = 256;
        let height = 240;
        let mut rgba = vec![0u8; width * height * 4];

        for (idx, &heat) in self.heat.iter().enumerate() {
            let pixel_idx = idx * 4;

            // Simple thermal gradient mapping (matches memory heatmap)
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

    #[test]
    fn should_initialize_heatmap() {
        let heatmap = SpriteHeatmap::new(0.9, 0.2);
        assert_eq!(heatmap.heat.len(), 256 * 240);
        assert_eq!(heatmap.decay_rate, 0.9);
        assert_eq!(heatmap.intensity, 0.2);
    }

    #[test]
    fn heatmap_records_and_decays() {
        let mut heatmap = SpriteHeatmap::new(0.5, 0.5);
        let mut core = NesCore::new();

        let mut dummy_page = [0xff; 256];
        dummy_page[0] = 50; // Y
        dummy_page[3] = 50; // X
        core.load_cpu_bytes(0x0200, &dummy_page);
        core.write_cpu_bus(0x4014, 0x02); // OAM DMA
        for _ in 0..180 {
            let _ = core.execute(crate::Command::StepCpu);
        }

        heatmap.record_frame(&core);

        // Assert sprite recorded heat
        let center_idx = 54 * 256 + 54;
        assert_eq!(heatmap.heat[center_idx], 0.5);

        // Assert cap
        heatmap.record_frame(&core);
        heatmap.record_frame(&core);
        assert_eq!(heatmap.heat[center_idx], 1.0);

        // Decay
        heatmap.decay_frame();
        assert_eq!(heatmap.heat[center_idx], 0.5);

        // Render BMP
        let bmp = heatmap.render_bmp().unwrap();
        assert!(bmp.starts_with(b"BM"), "BMP should start with magic bytes");
    }
}
