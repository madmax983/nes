//! Experimental visualizer for drawing sprite hitboxes on the framebuffer.

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSprite;

#[cfg(feature = "nova")]
/// A visualizer that draws 8x8 bounding boxes around active OAM sprites.
///
/// `HitboxVisualizer` reads the emulator's Object Attribute Memory (OAM),
/// decodes the sprites, and draws hollow neon rectangles directly onto the
/// raw RGBA framebuffer. This helps developers and speedrunners visually
/// verify object positions and collisions without relying on external tools.
pub struct HitboxVisualizer;

#[cfg(feature = "nova")]
impl HitboxVisualizer {
    /// Draws hitboxes for all valid sprites on the provided RGBA framebuffer.
    ///
    /// The `frame` must be a valid RGBA framebuffer slice of length `FRAME_WIDTH * FRAME_HEIGHT * 4`.
    pub fn draw_hitboxes(core: &NesCore, frame: &mut [u8]) {
        // Extract the 256-byte OAM memory
        let mut oam = [0_u8; 256];
        for i in 0..=255 {
            oam[usize::from(i)] = core.ppu_oam_byte(i);
        }

        // Iterate over up to 64 sprites (4 bytes each)
        for i in 0..64 {
            let sprite = OamSprite::from_oam(i, &oam);

            // Hide sprites off-screen (Y >= 240 is usually off-screen or hidden)
            if sprite.y >= FRAME_HEIGHT as u8 {
                continue;
            }

            // Draw an 8x8 hollow rectangle
            // A sprite is 8 pixels wide and 8 pixels tall by default.
            // (8x16 mode is not handled specifically here yet, Phase 1 only uses 8x8)
            Self::draw_rect(
                frame,
                sprite.x as usize,
                sprite.y as usize,
                8,
                8,
                [0, 255, 0, 255],
            );
        }
    }

    /// Draws a hollow rectangle directly on an RGBA framebuffer.
    fn draw_rect(frame: &mut [u8], x: usize, y: usize, w: usize, h: usize, color: [u8; 4]) {
        let max_x = x + w;
        let max_y = y + h;

        for current_y in y..max_y {
            if current_y >= FRAME_HEIGHT {
                continue;
            }
            for current_x in x..max_x {
                if current_x >= FRAME_WIDTH {
                    continue;
                }

                // Only draw border (hollow rectangle)
                if current_x == x
                    || current_x == max_x - 1
                    || current_y == y
                    || current_y == max_y - 1
                {
                    let base_idx = (current_y * FRAME_WIDTH + current_x) * 4;
                    // Safety check against buffer overflow
                    if base_idx + 3 < frame.len() {
                        frame[base_idx] = color[0]; // R
                        frame[base_idx + 1] = color[1]; // G
                        frame[base_idx + 2] = color[2]; // B
                        frame[base_idx + 3] = color[3]; // A
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::constants::FRAME_RGBA_BYTES;

    #[test]
    fn test_hitbox_visualizer_modifies_framebuffer() {
        let mut core = NesCore::new();
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];

        // Push a dummy sprite into OAM at index 0 (Y: 10, Tile: 0, Attr: 0, X: 20)
        // OAM is not directly writable in NesCore easily via an API for testing without
        // cpu writes or ppu manipulation, so we might need to mock it or inject it.
        // Actually, we can write via CPU bus to OAM_DATA (0x2004).
        // Let's set OAM_ADDR (0x2003) to 0.
        core.write_cpu_bus(0x2003, 0x00);
        core.write_cpu_bus(0x2004, 10); // Y = 10
        core.write_cpu_bus(0x2004, 0); // Tile = 0
        core.write_cpu_bus(0x2004, 0); // Attr = 0
        core.write_cpu_bus(0x2004, 20); // X = 20

        // Before drawing hitboxes, the pixel at (20, 10) is 0
        let base_idx = (10 * FRAME_WIDTH + 20) * 4;
        assert_eq!(frame[base_idx + 1], 0); // Green channel is 0

        HitboxVisualizer::draw_hitboxes(&core, &mut frame);

        // After drawing hitboxes, the pixel at (20, 10) should be green (255)
        assert_eq!(frame[base_idx], 0); // R
        assert_eq!(frame[base_idx + 1], 255); // G
        assert_eq!(frame[base_idx + 2], 0); // B
        assert_eq!(frame[base_idx + 3], 255); // A
    }
}
