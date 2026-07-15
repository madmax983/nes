//! Experimental real-time PPU state visualizer.
//!
//! This module provides tools to decode and visualize PPU internal state, including
//! pattern tables (CHR) and nametables (Background), enabling visual debugging of
//! graphics rendering.

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
/// A visualizer for PPU Pattern Tables (CHR ROM/RAM) and Nametables.
pub struct PpuVisualizer;

#[cfg(feature = "nova")]
impl PpuVisualizer {
    /// Renders the left pattern table (0x0000 - 0x0FFF) to an RGBA buffer.
    ///
    /// The buffer must be at least 128x128 pixels (128 * 128 * 4 = 65536 bytes).
    /// Palette 0 is used to colorize the tiles.
    pub fn render_pattern_table_left(core: &NesCore, buffer: &mut [u8]) {
        Self::render_pattern_table(core, buffer, 0x0000);
    }

    /// Renders the right pattern table (0x1000 - 0x1FFF) to an RGBA buffer.
    ///
    /// The buffer must be at least 128x128 pixels (128 * 128 * 4 = 65536 bytes).
    /// Palette 0 is used to colorize the tiles.
    pub fn render_pattern_table_right(core: &NesCore, buffer: &mut [u8]) {
        Self::render_pattern_table(core, buffer, 0x1000);
    }

    fn render_pattern_table(core: &NesCore, buffer: &mut [u8], base_address: u16) {
        if buffer.len() < 128 * 128 * 4 {
            return;
        }

        for tile_y in 0..16 {
            for tile_x in 0..16 {
                let tile_idx = tile_y * 16 + tile_x;
                let address = base_address + (tile_idx * 16);

                for row in 0..8 {
                    // Safe PPU read bypassing side-effects if needed.
                    let low_byte = core.ppu_peek_memory(address + row);
                    let high_byte = core.ppu_peek_memory(address + row + 8);

                    for col in 0..8 {
                        let bit_low = (low_byte >> (7 - col)) & 1;
                        let bit_high = (high_byte >> (7 - col)) & 1;
                        let color_idx = (bit_high << 1) | bit_low;

                        let pixel_x = tile_x as usize * 8 + col as usize;
                        let pixel_y = tile_y as usize * 8 + row as usize;
                        let buf_idx = (pixel_y * 128 + pixel_x) * 4;

                        // Use palette 0
                        let nes_color = core.ppu_peek_memory(0x3F00 + color_idx as u16);
                        let rgba = Self::nes_color_to_rgba(nes_color);

                        buffer[buf_idx] = rgba[0];
                        buffer[buf_idx + 1] = rgba[1];
                        buffer[buf_idx + 2] = rgba[2];
                        buffer[buf_idx + 3] = rgba[3];
                    }
                }
            }
        }
    }

    /// Temporary hardcoded palette mapping for visualizer.
    fn nes_color_to_rgba(color_idx: u8) -> [u8; 4] {
        let color_idx = color_idx & 0x3F;
        // Simple greyscale mapping for now, to ensure something is visible
        let val = (color_idx as u16 * 255 / 63) as u8;
        [val, val, val, 255]
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_ppu_visualizer_pattern_table_bounds() {
        let core = NesCore::new();
        let mut buffer = vec![0; 128 * 128 * 4];
        PpuVisualizer::render_pattern_table_left(&core, &mut buffer);
        PpuVisualizer::render_pattern_table_right(&core, &mut buffer);
    }
}
