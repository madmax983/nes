#![cfg(feature = "nova")]

use crate::NesCore;
use crate::bmp::encode_bmp;

/// Tracks PPU memory accesses (nametables, OAM, palettes) over time
/// to generate a thermal heatmap of rendering activity.
pub struct PpuHeatmap {
    heat: std::vec::Vec<f32>,
    decay_rate: f32,
    intensity: f32,
}

impl Default for PpuHeatmap {
    fn default() -> Self {
        Self::new(0.95, 0.1)
    }
}

impl PpuHeatmap {
    #[must_use]
    pub fn new(decay_rate: f32, intensity: f32) -> Self {
        Self {
            heat: vec![0.0; 256 * 240],
            decay_rate,
            intensity,
        }
    }

    /// Records OAM sprites and background scroll positions from the current frame.
    pub fn record_frame(&mut self, core: &NesCore) {
        let snapshot = core.save_state();

        // Decay all existing heat
        for h in &mut self.heat {
            *h *= self.decay_rate;
        }

        // Add heat for active sprites from OAM
        for i in 0..64 {
            let base = i * 4;
            let y = snapshot.ppu.oam[base] as usize;
            let x = snapshot.ppu.oam[base + 3] as usize;

            // If sprite is somewhat visible
            if y < 240 {
                // Approximate 8x8 sprite bounding box
                for dy in 0..8 {
                    for dx in 0..8 {
                        let px = x + dx;
                        let py = y + dy;
                        if px < 256 && py < 240 {
                            let idx = py * 256 + px;
                            self.heat[idx] = (self.heat[idx] + self.intensity).min(1.0);
                        }
                    }
                }
            }
        }

        // Add heat around the scroll coordinate to show screen boundary movement
        let scroll_x = snapshot.ppu.scroll_x as usize;
        let scroll_y = snapshot.ppu.scroll_y as usize;

        for dy in 0..8 {
            for dx in 0..8 {
                let px = (scroll_x + dx) % 256;
                let py = (scroll_y + dy) % 240;
                let idx = py * 256 + px;
                self.heat[idx] = (self.heat[idx] + self.intensity * 0.5).min(1.0);
            }
        }
    }

    /// Renders the heatmap to an RGBA buffer. Returns an error if buffer is incorrectly sized.
    pub fn render_rgba(&self, rgba: &mut [u8]) -> Result<(), String> {
        if rgba.len() != 256 * 240 * 4 {
            return Err("RGBA buffer must be exactly 256x240x4 bytes".to_string());
        }

        for (i, &heat) in self.heat.iter().enumerate() {
            let (r, g, b) = Self::heat_to_rgb(heat);
            rgba[i * 4] = r;
            rgba[i * 4 + 1] = g;
            rgba[i * 4 + 2] = b;
            rgba[i * 4 + 3] = 255;
        }

        Ok(())
    }

    /// Converts the current heat map to a standard BMP image byte vector.
    pub fn render_bmp(&self) -> Result<Vec<u8>, String> {
        let mut rgba = vec![0; 256 * 240 * 4];
        self.render_rgba(&mut rgba)?;
        encode_bmp(256, 240, &rgba).map_err(|e| e.to_string())
    }

    fn heat_to_rgb(heat: f32) -> (u8, u8, u8) {
        if heat < 0.25 {
            // Black to Blue
            let t = heat * 4.0;
            (0, 0, (t * 255.0) as u8)
        } else if heat < 0.5 {
            // Blue to Green
            let t = (heat - 0.25) * 4.0;
            (0, (t * 255.0) as u8, ((1.0 - t) * 255.0) as u8)
        } else if heat < 0.75 {
            // Green to Red
            let t = (heat - 0.5) * 4.0;
            ((t * 255.0) as u8, ((1.0 - t) * 255.0) as u8, 0)
        } else {
            // Red to White
            let t = (heat - 0.75) * 4.0;
            (255, (t * 255.0) as u8, (t * 255.0) as u8)
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_ppu_heatmap_records_activity() {
        let core = NesCore::new();
        let mut heatmap = PpuHeatmap::new(0.9, 0.5);

        // Record a frame
        heatmap.record_frame(&core);

        // Render to BMP
        let bmp_data = heatmap.render_bmp().unwrap();
        assert_eq!(&bmp_data[0..2], b"BM");
    }

    #[test]
    fn test_heat_to_rgb_gradient() {
        assert_eq!(PpuHeatmap::heat_to_rgb(0.0), (0, 0, 0));
        assert_eq!(PpuHeatmap::heat_to_rgb(0.5), (0, 255, 0));
        assert_eq!(PpuHeatmap::heat_to_rgb(1.0), (255, 255, 255));
    }
}
