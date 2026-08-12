#[cfg(feature = "nova")]
use crate::Button;
#[cfg(feature = "nova")]
use crate::constants::{FRAME_HEIGHT, FRAME_WIDTH};

#[cfg(feature = "nova")]
pub struct InputVisualizer;

#[cfg(feature = "nova")]
impl InputVisualizer {
    pub fn draw_gamepad(controller_bits: u8, frame: &mut [u8]) {
        if frame.len() != FRAME_WIDTH * FRAME_HEIGHT * 4 {
            return;
        }
        let draw_rect = |f: &mut [u8], x: usize, y: usize, w: usize, h: usize| {
            for cy in y..(y + h).min(FRAME_HEIGHT) {
                for cx in x..(x + w).min(FRAME_WIDTH) {
                    let idx = (cy * FRAME_WIDTH + cx) * 4;
                    if idx + 3 < f.len() {
                        f[idx] = 255;
                        f[idx + 1] = 0;
                        f[idx + 2] = 0;
                        f[idx + 3] = 255;
                    }
                }
            }
        };

        if (controller_bits & Button::Up.bit_mask()) != 0 {
            draw_rect(frame, 20, 210, 8, 8);
        }
        if (controller_bits & Button::Down.bit_mask()) != 0 {
            draw_rect(frame, 20, 226, 8, 8);
        }
        if (controller_bits & Button::Left.bit_mask()) != 0 {
            draw_rect(frame, 12, 218, 8, 8);
        }
        if (controller_bits & Button::Right.bit_mask()) != 0 {
            draw_rect(frame, 28, 218, 8, 8);
        }
        if (controller_bits & Button::Select.bit_mask()) != 0 {
            draw_rect(frame, 50, 222, 12, 4);
        }
        if (controller_bits & Button::Start.bit_mask()) != 0 {
            draw_rect(frame, 70, 222, 12, 4);
        }
        if (controller_bits & Button::B.bit_mask()) != 0 {
            draw_rect(frame, 100, 220, 8, 8);
        }
        if (controller_bits & Button::A.bit_mask()) != 0 {
            draw_rect(frame, 120, 220, 8, 8);
        }
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;

    #[test]
    fn test_draw_gamepad_modifies_framebuffer() {
        let mut frame = vec![0_u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
        InputVisualizer::draw_gamepad(crate::Button::A.bit_mask(), &mut frame);
        let has_color = frame.iter().any(|&byte| byte > 0);
        assert!(
            has_color,
            "Framebuffer should be modified when a button is pressed"
        );
    }
}
