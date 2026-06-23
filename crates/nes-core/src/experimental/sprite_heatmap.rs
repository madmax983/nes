//! Experimental visual heatmap generator for sprite screen coverage.
//!
//! Tracks where sprites are rendered on the screen over time and generates
//! a thermal heatmap BMP image. Useful for analyzing enemy pathing,
//! player movement patterns, or level hotspots.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;
#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSpatialQuery;

#[cfg(feature = "nova")]
pub struct SpriteHeatmap {
    heat: Vec<f32>,
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
    pub fn new(decay_rate: f32, intensity: f32) -> Self {
        Self {
            heat: vec![0.0; FRAME_WIDTH * FRAME_HEIGHT],
            decay_rate,
            intensity,
        }
    }

    pub fn record_frame(&mut self, core: &NesCore) {
        let query = OamSpatialQuery::new(core);
        for sprite in query.sprites() {
            // Sprites at y >= 240 are usually offscreen
            if sprite.y as usize >= FRAME_HEIGHT {
                continue;
            }

            let w = 8;
            let h = 8; // 8x16 not explicitly handled, basic 8x8 assumed

            let start_x = sprite.x as usize;
            let start_y = sprite.y as usize;

            for y in start_y..(start_y + h).min(FRAME_HEIGHT) {
                for x in start_x..(start_x + w).min(FRAME_WIDTH) {
                    let idx = y * FRAME_WIDTH + x;
                    self.heat[idx] = (self.heat[idx] + self.intensity).min(1.0);
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
        let mut rgba = vec![0u8; FRAME_WIDTH * FRAME_HEIGHT * 4];

        for (idx, &heat) in self.heat.iter().enumerate() {
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

            let pixel_idx = idx * 4;
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
    fn test_sprite_heatmap_recording_and_rendering() {
        let mut core = NesCore::new();
        let mut heatmap = SpriteHeatmap::new(0.9, 0.5);

        // Put a sprite on screen
        let mut dummy_page = [0xff; 256];
        dummy_page[0] = 50; // Y
        dummy_page[3] = 50; // X
        core.load_cpu_bytes(0x0200, &dummy_page);
        core.write_cpu_bus(0x4014, 0x02); // OAM DMA

        // Run DMA
        for _ in 0..180 {
            let _ = core.execute(Command::StepCpu);
        }

        heatmap.record_frame(&core);
        heatmap.decay_frame();

        let bmp = heatmap.render_bmp().unwrap();
        assert_eq!(&bmp[0..2], b"BM");

        // Assert some heat is applied
        let heat_val = heatmap.heat[50 * FRAME_WIDTH + 50];
        assert!(heat_val > 0.0);
    }
}
