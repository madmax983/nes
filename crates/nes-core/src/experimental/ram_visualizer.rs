//! Experimental tools for visualizing NES working RAM.
//!
//! This module provides utilities like [`RamVisualizer`] to extract the primary 2KB
//! working RAM (`$0000`-`$07FF`) and convert it into standard image formats (like BMP).
//! This acts as a macroscopic "memory map" where memory density and activity can be
//! seen visually, allowing for intuition around memory allocators or zero-page usage.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;

#[cfg(feature = "nova")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Palette choices for visualizing memory.
pub enum RamPalette {
    /// Maps 0-255 to grayscale.
    Grayscale,
    /// Maps 0 to black, and non-zero values to a color gradient based on value.
    Heatmap,
}

/// A utility for extracting Working RAM data from an NES core and converting it into image formats.
#[cfg(feature = "nova")]
pub struct RamVisualizer;

#[cfg(feature = "nova")]
impl RamVisualizer {
    /// Extracts the 2KB working RAM (`$0000`-`$07FF`) and encodes it as a BMP image.
    ///
    /// The resulting image is 64x32 pixels, where each pixel represents one byte of RAM.
    pub fn extract_ram_bmp(core: &NesCore, palette: RamPalette) -> Result<Vec<u8>, String> {
        let width = 64;
        let height = 32;
        let ram_size = 2048;
        let mut rgba = Vec::with_capacity(ram_size * 4);

        for addr in 0..ram_size {
            let val = core.read_memory(addr as u16);
            let color = match palette {
                RamPalette::Grayscale => [val, val, val, 255],
                RamPalette::Heatmap => {
                    if val == 0 {
                        [0, 0, 0, 255]
                    } else if val < 85 {
                        [0, 0, 255, 255]
                    } else if val < 170 {
                        [255, 255, 0, 255]
                    } else {
                        [255, 0, 0, 255]
                    }
                }
            };
            rgba.extend_from_slice(&color);
        }

        encode_bmp(width, height, &rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_ram_visualizer() {
        let mut core = NesCore::new();
        // Set some dummy values in zero page
        core.write_cpu_bus(0x0000, 0xFF);
        core.write_cpu_bus(0x0001, 0x80);

        let bmp_bytes = RamVisualizer::extract_ram_bmp(&core, RamPalette::Grayscale).unwrap();

        assert!(bmp_bytes.len() > 54); // BMP header size
        assert_eq!(&bmp_bytes[0..2], b"BM");
    }

    #[test]
    fn test_ram_visualizer_heatmap() {
        let mut core = NesCore::new();
        // Set values that trigger all 4 branches of the heatmap logic
        core.write_cpu_bus(0x0000, 0); // Black
        core.write_cpu_bus(0x0001, 50); // Blue
        core.write_cpu_bus(0x0002, 100); // Yellow
        core.write_cpu_bus(0x0003, 200); // Red

        let bmp_bytes = RamVisualizer::extract_ram_bmp(&core, RamPalette::Heatmap).unwrap();

        assert!(bmp_bytes.len() > 54); // BMP header size
        assert_eq!(&bmp_bytes[0..2], b"BM");
    }
}
