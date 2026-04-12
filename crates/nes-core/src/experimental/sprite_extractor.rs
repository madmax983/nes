//! Experimental tools for extracting sprite and tile data directly from the emulator's memory.
//!
//! This module provides utilities like [`SpriteExtractor`] to parse CHR-RAM data and convert it
//! into recognizable image formats (such as BMP files). It is primarily useful for debugging,
//! automated sprite sheet generation, or external visualization tools that need to inspect
//! graphics state without manually reading individual bytes.

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
        let mut core = NesCore::new();
        // Load some dummy bytes so we have non-zero data
        let mut snapshot = core.save_state();
        // Modify some CHR RAM bytes
        snapshot.ppu.chr[0] = 0b10101010; // plane 0, row 0
        snapshot.ppu.chr[8] = 0b11001100; // plane 1, row 0
        core.load_state(&snapshot);

        let bmp_data = SpriteExtractor::extract_chr_ram_bmp(&core).unwrap();

        // Check BMP header magic
        assert_eq!(&bmp_data[0..2], b"BM");

        // Assert on the correct total image size:
        // A 128x256 image with 24-bit depth should be:
        // Header: 54 bytes
        // Pixel data: 128 * 256 * 3 = 98304 bytes
        // Total: 98358 bytes
        assert_eq!(bmp_data.len(), 98358);
    }

    #[test]
    fn test_grayscale_palette_mapping() {
        let mut core = NesCore::new();
        let mut snapshot = core.save_state();

        // Create 4 pixels with different color indices in the first tile, first row
        // Bit patterns for color indices: 0, 1, 2, 3
        // plane0 needs bits: 0, 1, 0, 1 -> 0b0101_0000
        // plane1 needs bits: 0, 0, 1, 1 -> 0b0011_0000
        snapshot.ppu.chr[0] = 0b01010000;
        snapshot.ppu.chr[8] = 0b00110000;
        core.load_state(&snapshot);

        let bmp_data = SpriteExtractor::extract_chr_ram_bmp(&core).unwrap();

        // Last row in BMP corresponds to first row in CHR.
        // offset: 54 + 255 * (128 * 3) = 97974
        let row_start = 97974;

        // Color 0: (0, 0)
        assert_eq!(&bmp_data[row_start..row_start + 3], &[0, 0, 0]);
        // Color 1: (0, 1) -> grayscale 85
        assert_eq!(&bmp_data[row_start + 3..row_start + 6], &[85, 85, 85]);
        // Color 2: (1, 0) -> grayscale 170
        assert_eq!(&bmp_data[row_start + 6..row_start + 9], &[170, 170, 170]);
        // Color 3: (1, 1) -> grayscale 255
        assert_eq!(&bmp_data[row_start + 9..row_start + 12], &[255, 255, 255]);
    }

    #[test]
    fn test_extract_chr_ram_bmp_mutants() {
        let mut core = NesCore::new();
        let mut snapshot = core.save_state();

        // Ensure tiles other than 0 are mapped correctly. Let's use tile_idx 1
        // tile_y = 0, tile_x = 1.
        // address = 16.
        snapshot.ppu.chr[16] = 0b10000000; // 1 pixel with color 1 at col 0.
        // tile_y = 1, tile_x = 0
        // tile_idx = 16. address = 256.
        snapshot.ppu.chr[256] = 0b01000000; // 1 pixel with color 1 at col 1.

        core.load_state(&snapshot);

        let bmp_data = SpriteExtractor::extract_chr_ram_bmp(&core).unwrap();

        // bmp_data[54] is the top-left pixel, actually it's bottom up.
        // row 0 is at offset: 54 + 255 * (128 * 3) = 97974
        // tile_x = 1 -> pixel x = 8.
        let tile_1_offset = 97974 + 8 * 3;
        assert_eq!(&bmp_data[tile_1_offset..tile_1_offset + 3], &[85, 85, 85]);

        // tile_y = 1 -> pixel y = 8.
        // BMP row 255 - 8 = 247.
        // offset: 54 + 247 * 384 = 54 + 94848 = 94902.
        // col = 1 -> pixel x = 1.
        let tile_16_offset = 94902 + 3;
        assert_eq!(&bmp_data[tile_16_offset..tile_16_offset + 3], &[85, 85, 85]);
    }
}
