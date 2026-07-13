//! Experimental tool for rendering TAS (Tool-Assisted Speedrun) inputs as a visual piano roll.
//!
//! This module converts a `TasMovie` into a BMP image where the X-axis represents time (frames)
//! and the Y-axis represents button presses, creating a visual "piano roll" of the speedrun.

#[cfg(feature = "nova")]
use crate::Button;
#[cfg(feature = "nova")]
use crate::bmp::encode_bmp;
#[cfg(feature = "nova")]
use crate::tas::TasMovie;

#[cfg(feature = "nova")]
/// A visualizer that converts a sequence of TAS inputs into a Piano Roll image.
pub struct TasPianoRoll;

#[cfg(feature = "nova")]
impl TasPianoRoll {
    /// Renders a TAS movie as a piano roll BMP image.
    ///
    /// Each frame of the movie corresponds to 1 pixel on the X-axis.
    /// The Y-axis is divided into 8 bands, one for each NES controller button.
    /// Active button presses are drawn in bright neon green, while inactive areas are dark gray.
    ///
    /// # Parameters
    /// * `movie` - The TAS movie to visualize.
    /// * `max_width` - The maximum width (in frames) of the generated image. Useful to prevent OOM.
    ///
    /// # Returns
    /// A byte vector containing the encoded BMP image.
    pub fn render_bmp(movie: &TasMovie, max_width: usize) -> Result<Vec<u8>, String> {
        let total_frames: u32 = movie.runs().iter().map(|r| r.frames).sum();
        if total_frames == 0 {
            return Err("Movie is empty".to_string());
        }

        let width = (total_frames as usize).min(max_width);
        if width == 0 {
            return Err("Width must be greater than 0".to_string());
        }

        let band_height = 8;
        let height = 8 * band_height; // 8 buttons * 8 pixels = 64 pixels tall

        let mut rgba = vec![0u8; width * height * 4];

        // Background dark gray
        for px in rgba.chunks_exact_mut(4) {
            px[0] = 30; // R
            px[1] = 30; // G
            px[2] = 30; // B
            px[3] = 255; // A
        }

        let buttons = [
            Button::A,
            Button::B,
            Button::Select,
            Button::Start,
            Button::Up,
            Button::Down,
            Button::Left,
            Button::Right,
        ];

        let mut current_frame = 0;
        for run in movie.runs() {
            let p1_bits = run.controller1_bits;

            for _ in 0..run.frames {
                if current_frame >= width {
                    break;
                }

                for (btn_idx, button) in buttons.iter().enumerate() {
                    let is_pressed = (p1_bits & button.bit_mask()) != 0;

                    if is_pressed {
                        // Draw vertical line for this frame, within the button's band
                        let y_start = btn_idx * band_height;
                        for y in (y_start + 1)..(y_start + band_height - 1) {
                            // Leave 1px gap between bands
                            let idx = (y * width + current_frame) * 4;
                            // Neon Green (RGB: 57, 255, 20)
                            rgba[idx] = 57;
                            rgba[idx + 1] = 255;
                            rgba[idx + 2] = 20;
                            rgba[idx + 3] = 255;
                        }
                    }
                }
                current_frame += 1;
            }
            if current_frame >= width {
                break;
            }
        }

        encode_bmp(width, height, &rgba)
    }
}

#[cfg(all(test, feature = "nova", feature = "tas"))]
mod tests {
    use super::*;
    use crate::tas::TasFrameRun;

    #[test]
    fn test_render_empty_movie_returns_error() {
        let movie = TasMovie::default();
        let result = TasPianoRoll::render_bmp(&movie, 100);
        assert_eq!(result.unwrap_err(), "Movie is empty");
    }

    #[test]
    fn test_render_valid_movie_generates_bmp() {
        let run1 = TasFrameRun::new(Button::A.bit_mask(), 0, 10);
        let run2 = TasFrameRun::new(Button::A.bit_mask() | Button::Right.bit_mask(), 0, 20);
        let run3 = TasFrameRun::new(0, 0, 5); // Idle

        let movie = TasMovie::from_runs(vec![run1, run2, run3]);

        // Render up to 50 frames, but movie is only 35 frames long
        let bmp_bytes = TasPianoRoll::render_bmp(&movie, 50).unwrap();

        // Verify BMP header
        assert_eq!(&bmp_bytes[0..2], b"BM");
    }

    #[test]
    fn test_render_caps_at_max_width() {
        let run = TasFrameRun::new(Button::Start.bit_mask(), 0, 100);
        let movie = TasMovie::from_runs(vec![run]);

        let max_width = 42;
        let bmp_bytes = TasPianoRoll::render_bmp(&movie, max_width).unwrap();

        assert_eq!(&bmp_bytes[0..2], b"BM");
    }
}
