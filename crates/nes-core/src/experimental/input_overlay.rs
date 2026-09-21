#![cfg(feature = "nova")]

use crate::Button;
use crate::constants::{FRAME_HEIGHT, FRAME_RGBA_BYTES, FRAME_WIDTH};

/// Draws controller inputs directly onto a raw RGBA framebuffer.
pub struct InputOverlay;

#[cfg(feature = "nova")]
impl InputOverlay {
    /// Draws a visual representation of controller inputs on the framebuffer.
    pub fn draw(framebuffer: &mut [u8], controller_bits: u8, start_x: usize, start_y: usize) {
        if framebuffer.len() < FRAME_RGBA_BYTES {
            return;
        }
        let buttons = [
            (Button::Up, 10, 0),
            (Button::Left, 0, 10),
            (Button::Right, 20, 10),
            (Button::Down, 10, 20),
            (Button::Select, 40, 15),
            (Button::Start, 60, 15),
            (Button::B, 90, 15),
            (Button::A, 110, 15),
        ];
        for (button, ox, oy) in buttons {
            let pressed = (controller_bits & button.bit_mask()) != 0;
            let color = if pressed {
                [255, 0, 0, 255]
            } else {
                [100, 100, 100, 128]
            };
            for dy in 0..8 {
                for dx in 0..8 {
                    let px = start_x + ox + dx;
                    let py = start_y + oy + dy;
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
    fn test_draw_button_pressed() {
        let mut fb = vec![0; FRAME_RGBA_BYTES];
        InputOverlay::draw(&mut fb, Button::A.bit_mask(), 10, 10);
        let idx = ((10 + 15) * FRAME_WIDTH + (10 + 110)) * 4;
        assert_eq!(fb[idx], 255);
    }

    #[test]
    fn test_bounds_check() {
        let mut fb = vec![0; 100];
        InputOverlay::draw(&mut fb, Button::A.bit_mask(), 0, 0);
    }
}
