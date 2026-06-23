//! Experimental visual overlay for controller inputs.
//!
//! This module provides a way to render a real-time display of controller inputs
//! directly onto the NES framebuffer. This is extremely useful for speedrun
//! recordings, TAS playback verification, or streaming, allowing viewers to see
//! exactly what buttons are being pressed at any given frame.

#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};

#[cfg(feature = "nova")]
/// An experimental overlay that renders controller state onto a framebuffer.
pub struct InputOverlay;

#[cfg(feature = "nova")]
impl InputOverlay {
    /// Draws the current controller state onto the given RGBA framebuffer.
    ///
    /// The overlay is drawn at the bottom right of the screen.
    /// `controller_bits` should be the standard 8-bit NES controller state.
    pub fn draw(framebuffer: &mut [u8], controller_bits: u8) {
        if framebuffer.len() != FRAME_RGBA_BYTES {
            return;
        }

        // Bit 7: Right, Bit 6: Left, Bit 5: Down, Bit 4: Up, Bit 3: Start, Bit 2: Select, Bit 1: B, Bit 0: A
        let masks = [
            0x80, // R
            0x40, // L
            0x20, // D
            0x10, // U
            0x08, // Start
            0x04, // Select
            0x02, // B
            0x01, // A
        ];

        let box_size = 6;
        let spacing = 2;
        let total_width = masks.len() * (box_size + spacing);

        let start_x = FRAME_WIDTH.saturating_sub(total_width + 4);
        let start_y = FRAME_HEIGHT.saturating_sub(box_size + 4);

        for (i, &mask) in masks.iter().enumerate() {
            let pressed = (controller_bits & mask) != 0;
            // Bright red for pressed, dark gray for unpressed
            let color = if pressed {
                [255, 50, 50, 255]
            } else {
                [50, 50, 50, 200]
            };

            let x = start_x + i * (box_size + spacing);
            let y = start_y;

            for dy in 0..box_size {
                for dx in 0..box_size {
                    let px = x + dx;
                    let py = y + dy;
                    if px < FRAME_WIDTH && py < FRAME_HEIGHT {
                        let idx = (py * FRAME_WIDTH + px) * 4;
                        if idx + 3 < framebuffer.len() {
                            framebuffer[idx] = color[0];
                            framebuffer[idx + 1] = color[1];
                            framebuffer[idx + 2] = color[2];
                            framebuffer[idx + 3] = color[3];
                        }
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
    fn test_input_overlay_draws_pressed_buttons_red() {
        let mut framebuffer = vec![0; FRAME_RGBA_BYTES];
        // Press A (0x01)
        InputOverlay::draw(&mut framebuffer, 0x01);

        // Find a red pixel
        let mut found_red = false;
        for i in (0..FRAME_RGBA_BYTES).step_by(4) {
            if framebuffer[i] == 255 && framebuffer[i + 1] == 50 && framebuffer[i + 2] == 50 {
                found_red = true;
                break;
            }
        }
        assert!(
            found_red,
            "Expected to find red pixels from pressed A button"
        );
    }

    #[test]
    fn test_input_overlay_draws_unpressed_buttons_gray() {
        let mut framebuffer = vec![0; FRAME_RGBA_BYTES];
        // Press A (0x01), so B through Right are unpressed
        InputOverlay::draw(&mut framebuffer, 0x01);

        let mut found_gray = false;
        for i in (0..FRAME_RGBA_BYTES).step_by(4) {
            if framebuffer[i] == 50 && framebuffer[i + 1] == 50 && framebuffer[i + 2] == 50 {
                found_gray = true;
                break;
            }
        }
        assert!(
            found_gray,
            "Expected to find gray pixels from unpressed buttons"
        );
    }

    #[test]
    fn test_input_overlay_ignores_invalid_framebuffer() {
        let mut small_buffer = vec![0; 10];
        InputOverlay::draw(&mut small_buffer, 0xFF);
        // Should not panic, and buffer should remain untouched
        assert_eq!(small_buffer, vec![0; 10]);
    }
}
