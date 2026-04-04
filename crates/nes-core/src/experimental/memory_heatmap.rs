//! Experimental visual heatmap generator for CPU memory access.
//!
//! This module tracks memory accesses over time and renders a thermal heatmap
//! image of the 64KB memory space, useful for profiling and debugging.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;
#[cfg(feature = "nova")]
use crate::cpu::CpuBusAccessKind;

#[cfg(feature = "nova")]
/// Tracks CPU memory accesses to generate a visual heatmap.
pub struct MemoryHeatmap {
    heat: std::vec::Vec<f32>,
    decay_rate: f32,
    intensity: f32,
}

#[cfg(feature = "nova")]
impl Default for MemoryHeatmap {
    fn default() -> Self {
        Self::new(0.95, 0.1)
    }
}

#[cfg(feature = "nova")]
impl MemoryHeatmap {
    /// Creates a new heatmap tracker.
    ///
    /// * `decay_rate` - How quickly heat dissipates each frame (e.g. 0.95 = 5% decay).
    /// * `intensity` - How much heat is added per memory access.
    #[must_use]
    pub fn new(decay_rate: f32, intensity: f32) -> Self {
        Self {
            heat: vec![0.0; 65536],
            decay_rate,
            intensity,
        }
    }

    /// Records all memory accesses from the core's latest CPU trace.
    pub fn record_trace(&mut self, core: &NesCore) {
        for access in core.last_cpu_bus_trace() {
            let addr = access.addr as usize;
            match access.kind {
                CpuBusAccessKind::Read | CpuBusAccessKind::Write => {
                    self.heat[addr] = (self.heat[addr] + self.intensity).min(1.0);
                }
                CpuBusAccessKind::DummyRead => {}
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

    /// Renders the current heatmap as a 256x256 BMP image.
    /// Hot addresses are red, warm are yellow, cool are blue.
    pub fn render_bmp(&self) -> Result<std::vec::Vec<u8>, String> {
        let width = 256;
        let height = 256;
        let mut rgba = vec![0u8; width * height * 4];

        for (addr, &heat) in self.heat.iter().enumerate() {
            let x = addr % 256;
            let y = addr / 256;
            let idx = (y * width + x) * 4;

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
    use crate::Command;
    use crate::NesCore;

    #[test]
    fn heatmap_records_and_decays() {
        let mut core = NesCore::new();
        core.set_trace_enabled(true);
        let _ = core.execute(Command::StepFrame);

        let mut heatmap = MemoryHeatmap::default();
        heatmap.record_trace(&core);
        heatmap.decay_frame();

        // Heatmap should render without error
        let bmp = heatmap.render_bmp().unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }

    #[test]
    fn heatmap_covers_color_bands_and_dummy_reads() {
        let mut heatmap = MemoryHeatmap::new(0.5, 0.1);

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

        // Cover dummy reads
        let mut core = NesCore::new();
        core.set_trace_enabled(true);
        let _ = core.execute(Command::StepFrame); // Causes dummy reads internally
        heatmap.record_trace(&core);
    }
}
