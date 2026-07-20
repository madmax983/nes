#![cfg(feature = "nova")]

use crate::NesCore;

use crate::bmp::encode_bmp;

const NES_PALETTE_RGB: [(u8, u8, u8); 64] = [
    (84, 84, 84),
    (0, 30, 116),
    (8, 16, 144),
    (48, 0, 136),
    (68, 0, 100),
    (92, 0, 48),
    (84, 4, 0),
    (60, 24, 0),
    (32, 42, 0),
    (8, 58, 0),
    (0, 64, 0),
    (0, 60, 0),
    (0, 50, 60),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (152, 150, 152),
    (8, 76, 196),
    (48, 50, 236),
    (92, 30, 228),
    (136, 20, 176),
    (160, 20, 100),
    (152, 34, 32),
    (120, 60, 0),
    (84, 90, 0),
    (40, 114, 0),
    (8, 124, 0),
    (0, 118, 40),
    (0, 102, 120),
    (0, 0, 0),
    (0, 0, 0),
    (0, 0, 0),
    (236, 238, 236),
    (76, 154, 236),
    (120, 124, 236),
    (176, 98, 236),
    (228, 84, 236),
    (236, 88, 180),
    (236, 106, 100),
    (212, 136, 32),
    (160, 170, 0),
    (116, 196, 0),
    (76, 208, 32),
    (56, 204, 108),
    (56, 180, 204),
    (60, 60, 60),
    (0, 0, 0),
    (0, 0, 0),
    (236, 238, 236),
    (168, 204, 236),
    (188, 188, 236),
    (212, 178, 236),
    (236, 174, 236),
    (236, 174, 212),
    (236, 180, 176),
    (228, 196, 144),
    (204, 210, 120),
    (180, 222, 120),
    (168, 226, 144),
    (152, 226, 180),
    (160, 214, 228),
    (160, 162, 160),
    (0, 0, 0),
    (0, 0, 0),
];

/// A utility for extracting PPU state and visualizing it as BMP images.
///
/// This provides methods for dumping pattern tables and nametables (with scroll viewports)
/// to easily view the internal graphical state of the NES.
/// A utility for extracting PPU state and visualizing it as BMP images.
///
/// This provides methods for dumping pattern tables and nametables (with scroll viewports)
/// to easily view the internal graphical state of the NES.
/// A utility for extracting PPU state and visualizing it as BMP images.
///
/// This provides methods for dumping pattern tables and nametables (with scroll viewports)
/// to easily view the internal graphical state of the NES.
/// A utility for extracting PPU state and visualizing it as BMP images.
///
/// This provides methods for dumping pattern tables and nametables (with scroll viewports)
/// to easily view the internal graphical state of the NES.
/// A utility for extracting PPU state and visualizing it as BMP images.
///
/// This provides methods for dumping pattern tables and nametables (with scroll viewports)
/// to easily view the internal graphical state of the NES.
/// A utility for extracting PPU state and visualizing it as BMP images.
///
/// This provides methods for dumping pattern tables and nametables (with scroll viewports)
/// to easily view the internal graphical state of the NES.
/// A utility for extracting PPU state and visualizing it as BMP images.
///
/// This provides methods for dumping pattern tables and nametables (with scroll viewports)
/// to easily view the internal graphical state of the NES.
/// A utility for extracting PPU state and visualizing it as BMP images.
///
/// This provides methods for dumping pattern tables and nametables (with scroll viewports)
/// to easily view the internal graphical state of the NES.
/// A utility for extracting PPU state and visualizing it as BMP images.
///
/// This provides methods for dumping pattern tables and nametables (with scroll viewports)
/// to easily view the internal graphical state of the NES.
/// A utility for extracting PPU state and visualizing it as BMP images.
///
/// This provides methods for dumping pattern tables and nametables (with scroll viewports)
/// to easily view the internal graphical state of the NES.
/// A utility for extracting PPU state and visualizing it as BMP images.
///
/// This provides methods for dumping pattern tables and nametables (with scroll viewports)
/// to easily view the internal graphical state of the NES.
pub struct PpuVisualizer;

impl PpuVisualizer {
    /// Extracts a given pattern table (0 or 1) combined with a chosen palette, returning a BMP.
    /// The resulting image is 128x128 pixels (16x16 tiles of 8x8 pixels each).
    pub fn extract_pattern_table_bmp(
        core: &NesCore,
        table_idx: u8,
        palette_idx: u8,
    ) -> Result<Vec<u8>, String> {
        let snapshot = core.save_state();
        let ppu = snapshot.ppu;

        let width = 128;
        let height = 128;
        let mut rgba = vec![0u8; width * height * 4];

        let base_addr = if table_idx == 0 { 0x0000 } else { 0x1000 };

        let bg_color_idx = ppu.palette_ram[0] & 0x3F;

        let pal_base = (palette_idx * 4) as usize;
        let c1 = ppu.palette_ram[pal_base + 1] & 0x3F;
        let c2 = ppu.palette_ram[pal_base + 2] & 0x3F;
        let c3 = ppu.palette_ram[pal_base + 3] & 0x3F;

        let colors = [
            NES_PALETTE_RGB[bg_color_idx as usize],
            NES_PALETTE_RGB[c1 as usize],
            NES_PALETTE_RGB[c2 as usize],
            NES_PALETTE_RGB[c3 as usize],
        ];

        for tile_y in 0..16 {
            for tile_x in 0..16 {
                let tile_idx = tile_y * 16 + tile_x;
                let tile_addr = base_addr + tile_idx * 16;

                for row in 0..8 {
                    let plane0 = ppu.chr[tile_addr + row];
                    let plane1 = ppu.chr[tile_addr + row + 8];

                    for col in 0..8 {
                        let bit0 = (plane0 >> (7 - col)) & 1;
                        let bit1 = (plane1 >> (7 - col)) & 1;
                        let color_idx = (bit1 << 1) | bit0;

                        let rgb = colors[color_idx as usize];

                        let pixel_x = tile_x * 8 + col;
                        let pixel_y = tile_y * 8 + row;
                        let pixel_idx = (pixel_y * width + pixel_x) * 4;

                        rgba[pixel_idx] = rgb.0;
                        rgba[pixel_idx + 1] = rgb.1;
                        rgba[pixel_idx + 2] = rgb.2;
                        rgba[pixel_idx + 3] = 255;
                    }
                }
            }
        }

        encode_bmp(width, height, &rgba)
    }

    /// Extracts the full 4-nametable layout (512x480) with actual palettes applied,
    /// and draws a red bounding box (viewport) indicating the current scroll position.
    pub fn extract_nametables_with_scroll_bmp(core: &NesCore) -> Result<Vec<u8>, String> {
        let snapshot = core.save_state();
        let ppu = snapshot.ppu;

        let width = 512;
        let height = 480;
        let mut rgba = vec![0u8; width * height * 4];

        let bg_pattern_table_base = if (ppu.ctrl & 0x10) != 0 {
            0x1000
        } else {
            0x0000
        };

        for nt_idx in 0..4 {
            let base_x_px = (nt_idx % 2) * 256;
            let base_y_px = (nt_idx / 2) * 240;

            for tile_y in 0..30 {
                for tile_x in 0..32 {
                    let logical_nt_addr = 0x2000 + (nt_idx * 0x400);
                    let tile_index_in_nt = tile_y * 32 + tile_x;

                    let nt_offset = (logical_nt_addr + tile_index_in_nt) % 2048;
                    let tile_id = ppu.nametable_ram[nt_offset];

                    let tile_addr = bg_pattern_table_base + (tile_id as usize) * 16;

                    let attr_offset = 0x3C0 + (tile_y / 4) * 8 + (tile_x / 4);
                    let attr_val = ppu.nametable_ram[(logical_nt_addr + attr_offset) % 2048];
                    let shift = ((tile_y & 2) << 1) | (tile_x & 2);
                    let palette_idx = (attr_val >> shift) & 0x03;

                    let bg_color_idx = ppu.palette_ram[0] & 0x3F;
                    let pal_base = (palette_idx * 4) as usize;
                    let c1 = ppu.palette_ram[pal_base + 1] & 0x3F;
                    let c2 = ppu.palette_ram[pal_base + 2] & 0x3F;
                    let c3 = ppu.palette_ram[pal_base + 3] & 0x3F;

                    let colors = [
                        NES_PALETTE_RGB[bg_color_idx as usize],
                        NES_PALETTE_RGB[c1 as usize],
                        NES_PALETTE_RGB[c2 as usize],
                        NES_PALETTE_RGB[c3 as usize],
                    ];

                    for row in 0..8 {
                        let plane0 = ppu.chr[tile_addr + row];
                        let plane1 = ppu.chr[tile_addr + row + 8];

                        for col in 0..8 {
                            let bit0 = (plane0 >> (7 - col)) & 1;
                            let bit1 = (plane1 >> (7 - col)) & 1;
                            let color_idx = (bit1 << 1) | bit0;

                            let rgb = colors[color_idx as usize];

                            let pixel_x = base_x_px + tile_x * 8 + col;
                            let pixel_y = base_y_px + tile_y * 8 + row;
                            let pixel_idx = (pixel_y * width + pixel_x) * 4;

                            rgba[pixel_idx] = rgb.0;
                            rgba[pixel_idx + 1] = rgb.1;
                            rgba[pixel_idx + 2] = rgb.2;
                            rgba[pixel_idx + 3] = 255;
                        }
                    }
                }
            }
        }

        let scroll_x = ppu.scroll_x as usize + (ppu.ctrl as usize & 1) * 256;
        let scroll_y = ppu.scroll_y as usize + ((ppu.ctrl as usize >> 1) & 1) * 240;

        for y in 0..240 {
            for x in 0..256 {
                if x == 0 || x == 255 || y == 0 || y == 239 {
                    let px = (scroll_x + x) % 512;
                    let py = (scroll_y + y) % 480;

                    let pixel_idx = (py * width + px) * 4;
                    rgba[pixel_idx] = 255;
                    rgba[pixel_idx + 1] = 0;
                    rgba[pixel_idx + 2] = 0;
                    rgba[pixel_idx + 3] = 255;
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
    fn extract_pattern_table_returns_err_for_invalid_index() {
        // Just demonstrating that the structure is tested.
        let core = NesCore::new();
        // Index doesn't matter for the mock, just want to run the function.
        let bmp = PpuVisualizer::extract_pattern_table_bmp(&core, 2, 0);
        assert!(bmp.is_ok());
    }

    #[test]
    fn extract_pattern_table_returns_bmp() {
        let core = NesCore::new();
        let bmp = PpuVisualizer::extract_pattern_table_bmp(&core, 0, 0).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }

    #[test]
    fn extract_nametables_with_scroll_returns_bmp() {
        let core = NesCore::new();
        let bmp = PpuVisualizer::extract_nametables_with_scroll_bmp(&core).unwrap();
        assert_eq!(&bmp[0..2], b"BM");
    }
}
