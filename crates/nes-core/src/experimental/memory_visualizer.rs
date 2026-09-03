//! Experimental tools for visualizing the CPU memory map.

use alloc::{string::String, vec, vec::Vec};

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;

#[cfg(feature = "nova")]
/// A utility for rendering the entire CPU memory map as a visual image.
///
/// `MemoryVisualizer` exists to provide an immediate, color-coded macroscopic view of the
/// 64KB address space. This makes it easy for developers to spot uninitialized memory,
/// understand where specific segments (like PPU or PRG ROM) are mapped, and debug
/// mapper bank-switching behavior without staring at raw hex dumps.
pub struct MemoryVisualizer;

#[cfg(feature = "nova")]
impl MemoryVisualizer {
    /// Dumps the current NES CPU memory map as a 256x256 BMP image.
    ///
    /// This function visualizes the entire 64KB address space of the CPU.
    /// Each pixel represents a single byte of memory, where its coordinates (X, Y)
    /// map to the address `Y * 256 + X`. The colors indicate the region of memory:
    /// - Green: Internal RAM
    /// - Blue: PPU Registers
    /// - Yellow: APU and I/O Registers
    /// - Purple: Cartridge Expansion ROM
    /// - Cyan: PRG RAM
    /// - Grayscale: PRG ROM
    ///
    /// # Examples
    ///
    /// ```
    /// use nes_core::NesCore;
    /// use nes_core::experimental::memory_visualizer::MemoryVisualizer;
    ///
    /// let core = NesCore::new();
    /// let bmp_bytes = MemoryVisualizer::dump_memory_bmp(&core).unwrap();
    /// assert_eq!(&bmp_bytes[0..2], b"BM");
    /// ```
    ///
    /// # Errors
    /// Returns a `Result::Err` if the BMP encoding process fails (e.g., due to an invalid width/height combination, though hardcoded values here make this highly unlikely).
    pub fn dump_memory_bmp(core: &NesCore) -> Result<Vec<u8>, String> {
        let width = 256;
        let height = 256;
        let mut rgba = vec![0u8; width * height * 4];

        for addr in 0..=0xFFFF {
            let addr = addr as u16;
            let val = core.read_memory(addr);

            let x = addr % 256;
            let y = addr / 256;
            let idx = ((y as usize) * width + (x as usize)) * 4;

            let (r, g, b) = match addr {
                0x0000..=0x1FFF => (0, val, 0),     // RAM: Green
                0x2000..=0x3FFF => (0, 0, val),     // PPU: Blue
                0x4000..=0x401F => (val, val, 0),   // APU/IO: Yellow
                0x4020..=0x5FFF => (val, 0, val),   // Cartridge Expansion: Purple
                0x6000..=0x7FFF => (0, val, val),   // PRG RAM: Cyan
                0x8000..=0xFFFF => (val, val, val), // PRG ROM: Grayscale
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
    fn can_dump_memory_bmp() {
        let core = NesCore::new();
        let bmp_data = MemoryVisualizer::dump_memory_bmp(&core).unwrap();
        // Check BMP header magic
        assert_eq!(&bmp_data[0..2], b"BM");
    }
}
