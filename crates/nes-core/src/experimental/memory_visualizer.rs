#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;
#[cfg(feature = "nova")]
use crate::{CoreQuery, NesCore, api::QueryResult};

/// An experimental visualizer that generates BMP images from emulator memory.
///
/// This provides a quick way to "see" memory states (e.g., zero-page variables,
/// stack, or full RAM) as a heatmap or grayscale image, which is useful for
/// reverse engineering and AI/ML state representation.
#[cfg(feature = "nova")]
pub struct MemoryVisualizer;

#[cfg(feature = "nova")]
impl MemoryVisualizer {
    /// Renders the 2KB internal CPU RAM ($0000-$07FF) as a 64x32 BMP image.
    ///
    /// Each byte of memory is mapped to a grayscale pixel where `0x00` is black
    /// and `0xFF` is white.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_core::NesCore;
    /// use nes_core::experimental::memory_visualizer::MemoryVisualizer;
    ///
    /// let core = NesCore::new();
    /// let bmp_bytes = MemoryVisualizer::render_cpu_ram_bmp(&core).unwrap();
    /// assert_eq!(&bmp_bytes[0..2], b"BM");
    /// ```
    pub fn render_cpu_ram_bmp(core: &NesCore) -> Result<Vec<u8>, String> {
        let width = 64;
        let height = 32;
        let mut rgba = vec![0u8; width * height * 4];

        for i in 0..2048 {
            // Read 2KB work RAM ($0000-$07FF)
            let addr = i as u16;

            // Querying internal state safely per guidelines
            let val = match core.query(CoreQuery::Memory(addr)) {
                QueryResult::Memory(v) => v,
                _ => 0,
            };

            let pixel_idx = (i * 4) as usize;

            rgba[pixel_idx] = val; // R
            rgba[pixel_idx + 1] = val; // G
            rgba[pixel_idx + 2] = val; // B
            rgba[pixel_idx + 3] = 255; // A
        }

        encode_bmp(width, height, &rgba)
    }

    /// Renders the 1KB Name Tables ($2000-$23FF) as a 32x32 BMP image.
    ///
    /// The nametables hold the background tile indices. This generates a visual map
    /// of the active screen layout before tiles are even looked up.
    pub fn render_nametable_bmp(core: &NesCore) -> Result<Vec<u8>, String> {
        let width = 32;
        let height = 32;
        let mut rgba = vec![0u8; width * height * 4];

        for i in 0..1024 {
            let addr = 0x2000 + i as u16;
            let val = match core.query(CoreQuery::Memory(addr)) {
                QueryResult::Memory(v) => v,
                _ => 0,
            };

            let pixel_idx = (i * 4) as usize;

            // Use a color palette to make it more interesting than just grayscale
            // Tile indices are 0-255.
            let r = val.wrapping_mul(3);
            let g = val.wrapping_mul(5);
            let b = val.wrapping_mul(7);

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
    use crate::NesCore;

    #[test]
    fn can_render_cpu_ram_bmp() {
        let core = NesCore::new();
        let bmp_data = MemoryVisualizer::render_cpu_ram_bmp(&core).expect("Should render CPU RAM");
        assert_eq!(&bmp_data[0..2], b"BM", "BMP header magic is incorrect");
    }

    #[test]
    fn can_render_nametable_bmp() {
        let core = NesCore::new();
        let bmp_data =
            MemoryVisualizer::render_nametable_bmp(&core).expect("Should render nametable");
        assert_eq!(&bmp_data[0..2], b"BM", "BMP header magic is incorrect");
    }
}
