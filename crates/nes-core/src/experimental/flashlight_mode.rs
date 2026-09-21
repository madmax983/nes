//! Experimental visual filter that creates a "flashlight" or "spotlight" effect around a sprite.
//!
//! This module combines OAM (Object Attribute Memory) reading with framebuffer post-processing
//! to dynamically dim or black out areas of the screen that are far from a target sprite
//! (typically Sprite 0, which is often the player character).

#[cfg(feature = "nova")]
use crate::NesCore;
#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};
#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSprite;

#[cfg(feature = "nova")]
/// A visual filter that applies a spotlight effect around a specific OAM sprite.
pub struct FlashlightMode;

#[cfg(feature = "nova")]
impl FlashlightMode {
    /// Applies a flashlight effect to the provided RGBA framebuffer.
    ///
    /// The screen outside the `radius` of the `target_sprite_index` will be darkened.
    /// The `frame` must be a valid RGBA framebuffer slice of length `FRAME_WIDTH * FRAME_HEIGHT * 4`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// # use nes_core::NesCore;
    /// # use nes_core::constants::{FRAME_HEIGHT, FRAME_WIDTH};
    /// # use nes_core::experimental::flashlight_mode::FlashlightMode;
    /// let core = NesCore::new();
    /// let mut frame = vec![255_u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
    ///
    /// // Apply a flashlight of 40 pixels radius around Sprite 0
    /// FlashlightMode::apply(&core, &mut frame, 0, 40.0);
    /// ```
    pub fn apply(core: &NesCore, frame: &mut [u8], target_sprite_index: u8, radius: f32) {
        // Extract the 256-byte OAM memory
        let mut oam = [0_u8; 256];
        for i in 0..=255 {
            oam[usize::from(i)] = core.ppu_oam_byte(i);
        }

        // Get the target sprite
        let sprite = OamSprite::from_oam(target_sprite_index, &oam);

        // Sprite coordinates point to the top-left of the 8x8 sprite.
        // We calculate the center of the sprite.
        let center_x = sprite.x as f32 + 4.0;
        let center_y = sprite.y as f32 + 4.0;

        let radius_sq = radius * radius;

        for y in 0..FRAME_HEIGHT {
            let dy = y as f32 - center_y;
            let dy_sq = dy * dy;

            for x in 0..FRAME_WIDTH {
                let dx = x as f32 - center_x;
                let dist_sq = dx * dx + dy_sq;

                let base_idx = (y * FRAME_WIDTH + x) * 4;

                // If the pixel is outside the flashlight radius, dim it heavily.
                if dist_sq > radius_sq {
                    // Smooth falloff could be added, but a harsh cutoff fits the 8-bit era well.
                    // Let's do a 90% dim (multiply by 0.1) for a "spooky" effect.
                    if base_idx + 3 < frame.len() {
                        frame[base_idx] = (frame[base_idx] as f32 * 0.1) as u8;
                        frame[base_idx + 1] = (frame[base_idx + 1] as f32 * 0.1) as u8;
                        frame[base_idx + 2] = (frame[base_idx + 2] as f32 * 0.1) as u8;
                        // Alpha remains unchanged
                    }
                }
            }
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_flashlight_mode_dims_distant_pixels() {
        let mut core = NesCore::new();
        let mut frame = vec![255_u8; FRAME_WIDTH * FRAME_HEIGHT * 4];

        // Set OAM_ADDR to 0, then write Sprite 0 (Y, Tile, Attr, X)
        core.write_cpu_bus(0x2003, 0x00);
        core.write_cpu_bus(0x2004, 100); // Y = 100
        core.write_cpu_bus(0x2004, 0);   // Tile = 0
        core.write_cpu_bus(0x2004, 0);   // Attr = 0
        core.write_cpu_bus(0x2004, 100); // X = 100

        FlashlightMode::apply(&core, &mut frame, 0, 10.0);

        // Inside the radius (e.g. exactly at the center 104, 104) should remain 255
        let center_idx = (104 * FRAME_WIDTH + 104) * 4;
        assert_eq!(frame[center_idx], 255);

        // Outside the radius (e.g. 0, 0) should be dimmed to ~25 (255 * 0.1)
        let outside_idx = 0;
        assert_eq!(frame[outside_idx], 25);
    }
}
