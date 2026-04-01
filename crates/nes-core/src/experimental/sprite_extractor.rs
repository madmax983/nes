#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;

#[cfg(feature = "nova")]
pub struct SpriteExtractor;

#[cfg(feature = "nova")]
impl SpriteExtractor {
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

        // To test the `color_idx` match branches inside `extract_chr_ram_bmp`, we need to write
        // to the PPU's CHR memory. We can do this using `write_memory` by interacting with
        // PPUADDR ($2006) and PPUDATA ($2007).

        // We must load a mock ROM with CHR data directly, because the public API `NesCore`
        // doesn't expose a `write_memory` function for arbitrary locations like the PPU registers easily.

        // We can just create a snapshot, modify the CHR, and load it back.
        let mut snapshot = core.save_state();
        snapshot.ppu.chr[0] = 0b01010101;
        snapshot.ppu.chr[8] = 0b00110011;
        core.load_state(&snapshot);

        let bmp_data = SpriteExtractor::extract_chr_ram_bmp(&core).unwrap();

        // Check BMP header magic
        assert_eq!(&bmp_data[0..2], b"BM");

        // Output length is 128x256 * 3 + 54 = 98358
        assert_eq!(bmp_data.len(), 98358);
    }
}
