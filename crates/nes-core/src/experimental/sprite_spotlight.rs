#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSprite;

#[cfg(feature = "nova")]
/// A visualizer that highlights an area around a specific sprite, turning the rest of the screen to grayscale.
pub struct SpriteSpotlight;

#[cfg(feature = "nova")]
impl SpriteSpotlight {
    /// Applies the spotlight effect to a given RGBA framebuffer based on the sprite index and radius.
    pub fn apply_spotlight(core: &NesCore, frame: &mut [u8], target_sprite_index: u8, radius: f32) {
        #[allow(clippy::manual_is_multiple_of)]
        if frame.len() % 4 != 0 {
            return;
        }

        let mut oam = [0_u8; 256];
        for i in 0..=255 {
            oam[usize::from(i)] = core.ppu_oam_byte(i);
        }

        if target_sprite_index >= 64 {
            return;
        }

        let target_sprite = OamSprite::from_oam(target_sprite_index, &oam);
        if target_sprite.y >= FRAME_HEIGHT as u8 {
            // Target not on screen, turn everything grayscale
            Self::apply_grayscale(frame);
            return;
        }

        // Center of the 8x8 sprite
        let center_x = target_sprite.x as f32 + 4.0;
        let center_y = target_sprite.y as f32 + 4.0;
        let radius_sq = radius * radius;

        for y in 0..FRAME_HEIGHT {
            for x in 0..FRAME_WIDTH {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                if dx * dx + dy * dy > radius_sq {
                    let base_idx = (y * FRAME_WIDTH + x) * 4;
                    let r = u32::from(frame[base_idx]);
                    let g = u32::from(frame[base_idx + 1]);
                    let b = u32::from(frame[base_idx + 2]);
                    let luma = ((r * 77 + g * 150 + b * 29) >> 8) as u8;
                    frame[base_idx] = luma;
                    frame[base_idx + 1] = luma;
                    frame[base_idx + 2] = luma;
                }
            }
        }
    }

    fn apply_grayscale(frame: &mut [u8]) {
        for pixel in frame.chunks_exact_mut(4) {
            let r = u32::from(pixel[0]);
            let g = u32::from(pixel[1]);
            let b = u32::from(pixel[2]);
            let luma = ((r * 77 + g * 150 + b * 29) >> 8) as u8;
            pixel[0] = luma;
            pixel[1] = luma;
            pixel[2] = luma;
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::constants::FRAME_RGBA_BYTES;

    #[test]
    fn test_sprite_spotlight_applies_grayscale_outside_radius() {
        let mut core = NesCore::new();
        let mut frame = vec![0_u8; FRAME_RGBA_BYTES];

        // Fill frame with red
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = 255;
            pixel[1] = 0;
            pixel[2] = 0;
            pixel[3] = 255;
        }

        // Dummy sprite 0 at (10, 10)
        core.write_cpu_bus(0x2003, 0x00);
        core.write_cpu_bus(0x2004, 10);
        core.write_cpu_bus(0x2004, 0);
        core.write_cpu_bus(0x2004, 0);
        core.write_cpu_bus(0x2004, 10);

        SpriteSpotlight::apply_spotlight(&core, &mut frame, 0, 5.0);

        // Center pixel (14, 14) should remain red
        let center_idx = (14 * FRAME_WIDTH + 14) * 4;
        assert_eq!(frame[center_idx], 255);
        assert_eq!(frame[center_idx + 1], 0);

        // Far pixel (100, 100) should be grayscale
        let far_idx = (100 * FRAME_WIDTH + 100) * 4;
        assert_eq!(frame[far_idx], frame[far_idx + 1]); // Grayscale
    }
}
