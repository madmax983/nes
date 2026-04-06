//! Experimental tools for extracting and visualizing PPU nametables.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;
#[cfg(feature = "nova")]
use crate::rom::NametableMirroring;

#[cfg(feature = "nova")]
pub struct NametableViewer;

#[cfg(feature = "nova")]
impl NametableViewer {
    /// Extracts the full 4-nametable layout (512x480 pixels) as a BMP image.
    ///
    /// This resolves the internal nametable memory based on the current mirroring mode
    /// (Horizontal, Vertical, etc.) and maps the 8x8 tiles using CHR-RAM/ROM data.
    /// For simplicity in this experimental visualizer, tiles are mapped to a fixed
    /// grayscale palette rather than looking up the actual PPU palette ram.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use nes_core::NesCore;
    /// use nes_core::experimental::nametable_viewer::NametableViewer;
    ///
    /// let core = NesCore::new();
    /// let bmp_bytes = NametableViewer::extract_nametables_bmp(&core).unwrap();
    /// assert_eq!(&bmp_bytes[0..2], b"BM");
    /// ```
    pub fn extract_nametables_bmp(core: &NesCore) -> Result<Vec<u8>, String> {
        let snapshot = core.save_state();
        let ppu = snapshot.ppu;

        let width = 512;
        let height = 480;
        let mut rgba = vec![0u8; width * height * 4];

        // The background always uses pattern table 0 or 1 based on PPUCTRL bit 4.
        let bg_pattern_table_base = if (ppu.ctrl & 0x10) != 0 {
            0x1000
        } else {
            0x0000
        };

        for nt_idx in 0..4 {
            // Determine base coordinates for this nametable in the 512x480 image
            let base_x_px = (nt_idx % 2) * 256;
            let base_y_px = (nt_idx / 2) * 240;

            for tile_y in 0..30 {
                for tile_x in 0..32 {
                    // Resolve internal physical nametable address based on mirroring
                    let physical_nt_idx = match ppu.mirroring {
                        NametableMirroring::Horizontal => match nt_idx {
                            0 | 1 => 0,
                            2 | 3 => 1,
                            _ => unreachable!(),
                        },
                        NametableMirroring::Vertical => match nt_idx {
                            0 | 2 => 0,
                            1 | 3 => 1,
                            _ => unreachable!(),
                        },
                        NametableMirroring::OneScreenLower => 0,
                        NametableMirroring::OneScreenUpper => 1,
                    };

                    let nt_offset = physical_nt_idx * 1024;
                    let tile_index_in_nt = tile_y * 32 + tile_x;

                    // The tile ID is found in the nametable RAM
                    let tile_id = ppu.nametable_ram[nt_offset + tile_index_in_nt];

                    // Find pattern data address
                    let tile_addr = bg_pattern_table_base + (tile_id as usize) * 16;

                    // Read the 8x8 pixel tile
                    for row in 0..8 {
                        let plane0 = ppu.chr[tile_addr + row];
                        let plane1 = ppu.chr[tile_addr + row + 8];

                        for col in 0..8 {
                            let bit0 = (plane0 >> (7 - col)) & 1;
                            let bit1 = (plane1 >> (7 - col)) & 1;
                            let color_idx = (bit1 << 1) | bit0;

                            // Grayscale mapping
                            let color_val = match color_idx {
                                0 => 0,
                                1 => 85,
                                2 => 170,
                                3 => 255,
                                _ => unreachable!(),
                            };

                            let pixel_x = base_x_px + tile_x * 8 + col;
                            let pixel_y = base_y_px + tile_y * 8 + row;
                            let pixel_idx = (pixel_y * width + pixel_x) * 4;

                            rgba[pixel_idx] = color_val;
                            rgba[pixel_idx + 1] = color_val;
                            rgba[pixel_idx + 2] = color_val;
                            rgba[pixel_idx + 3] = 255;
                        }
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

    fn create_rom_with_mirroring(vertical: bool, mapper: u8) -> Vec<u8> {
        let mut rom = vec![0; 16 + 16384 + 8192];
        rom[0..4].copy_from_slice(b"NES\x1A");
        rom[4] = 1; // 1 PRG bank
        rom[5] = 1; // 1 CHR bank
        let mirror_bit = if vertical { 1 } else { 0 };
        rom[6] = (mapper << 4) | mirror_bit; // Mapper lower nybble + mirroring
        rom[7] = mapper & 0xF0; // Mapper upper nybble
        rom
    }

    #[test]
    fn test_horizontal_mirroring() {
        let mut core = NesCore::new();
        core.load_ines_rom(&create_rom_with_mirroring(false, 0))
            .unwrap();
        let bmp_data = NametableViewer::extract_nametables_bmp(&core).unwrap();
        assert_eq!(&bmp_data[0..2], b"BM");
    }

    #[test]
    fn test_vertical_mirroring() {
        let mut core = NesCore::new();
        core.load_ines_rom(&create_rom_with_mirroring(true, 0))
            .unwrap();
        let bmp_data = NametableViewer::extract_nametables_bmp(&core).unwrap();
        assert_eq!(&bmp_data[0..2], b"BM");
    }

    #[test]
    fn test_one_screen_mirroring() {
        let mut core = NesCore::new();
        // MMC1 (mapper 1) minimum PRG is 1 bank (16KB), but for our simplistic mock
        // sometimes 1 PRG bank isn't enough depending on mapper initialization logic.
        // Wait, MMC1 usually requires powers of two.
        // Let's use AxROM (Mapper 7) which supports OneScreen mirroring and is simpler.
        // AxROM expects 32KB PRG, so we provide 2 PRG banks.
        let mut rom = vec![0; 16 + 32768 + 8192];
        rom[0..4].copy_from_slice(b"NES\x1A");
        rom[4] = 2; // 2 PRG banks
        rom[5] = 1; // 1 CHR bank
        rom[6] = 7 << 4; // Mapper 7

        core.load_ines_rom(&rom).unwrap();

        // Default AxROM mirroring should be OneScreenLower (page 0)
        let bmp_data = NametableViewer::extract_nametables_bmp(&core).unwrap();
        assert_eq!(&bmp_data[0..2], b"BM");

        // Switch AxROM to OneScreenUpper via CPU write to $8000
        // AxROM: $8000-$FFFF bit 4 selects nametable page.
        core.write_cpu_bus(0x8000, 0x10);

        let bmp_data2 = NametableViewer::extract_nametables_bmp(&core).unwrap();
        assert_eq!(&bmp_data2[0..2], b"BM");
    }
}
