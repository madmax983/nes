//! Experimental tools for extracting sprite and tile data directly from the emulator's memory.
//!
//! This module provides utilities like [`crate::experimental::sprite_extractor::SpriteExtractor`] to parse CHR-RAM data and convert it
//! into recognizable image formats (such as BMP files). It is primarily useful for debugging,
//! automated sprite sheet generation, or external visualization tools that need to inspect
//! graphics state without manually reading individual bytes.

use alloc::{string::String, vec, vec::Vec};

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;

/// A utility for extracting CHR-RAM data from an NES core and converting it into image formats.
///
/// `SpriteExtractor` is an experimental feature that reads raw pattern tables from the PPU's memory
/// and maps the 2-bit NES color indices to a standard grayscale palette, allowing developers
/// to visualize the current tile set in standard formats like BMP.
#[cfg(feature = "nova")]
pub struct SpriteExtractor;

#[cfg(feature = "nova")]
impl SpriteExtractor {
    /// Extracts the entire CHR-RAM memory (pattern tables) and encodes it as a BMP image.
    ///
    /// This function converts the 8x8 pixel tiles stored in the NES core into a 128x256 pixel image.
    /// It is extremely useful for debugging the PPU memory visually. The 2-bit color indices
    /// are mapped to grayscale values (0, 85, 170, 255).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_core::NesCore;
    /// use nes_core::experimental::sprite_extractor::SpriteExtractor;
    ///
    /// let core = NesCore::new();
    /// // Extract the current sprite sheet from CHR-RAM
    /// let bmp_bytes = SpriteExtractor::extract_chr_ram_bmp(&core).unwrap();
    ///
    /// // The resulting bytes contain a valid BMP file starting with the "BM" magic bytes
    /// assert_eq!(&bmp_bytes[0..2], b"BM");
    /// ```
    pub fn extract_chr_ram_bmp(core: &NesCore) -> Result<Vec<u8>, String> {
        let snapshot = core.save_state();
        let chr = snapshot.ppu.chr;

        let width = 128; // 16 tiles across
        let height = 256; // 32 tiles down

        let mut rgba = vec![0u8; width * height * 4];

        for tile_y in 0..32 {
            for tile_x in 0..16 {
                let tile_idx = tile_y * 16 + tile_x;
                let tile_addr = tile_idx * 16;

                for row in 0..8 {
                    let plane0 = chr[tile_addr + row];
                    let plane1 = chr[tile_addr + row + 8];

                    for col in 0..8 {
                        let bit0 = (plane0 >> (7 - col)) & 1;
                        let bit1 = (plane1 >> (7 - col)) & 1;
                        let color_idx = (bit1 << 1) | bit0;

                        let base_x = tile_x * 8 + col;
                        let base_y = tile_y * 8 + row;

                        let pixel_idx = (base_y * width + base_x) * 4;

                        // Simple grayscale palette mapping for raw CHR
                        let color_val = match color_idx {
                            0 => 0,
                            1 => 85,
                            2 => 170,
                            3 => 255,
                            _ => unreachable!(),
                        };

                        rgba[pixel_idx] = color_val;
                        rgba[pixel_idx + 1] = color_val;
                        rgba[pixel_idx + 2] = color_val;
                        rgba[pixel_idx + 3] = 255;
                    }
                }
            }
        }

        encode_bmp(width, height, &rgba)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn can_extract_sprite_sheet() {
        let core = NesCore::new();
        let bmp_data = SpriteExtractor::extract_chr_ram_bmp(&core).unwrap();
        // Check BMP header magic
        assert_eq!(&bmp_data[0..2], b"BM");
    }
}
