//! Experimental screen shake effect triggered by audio volume.
//!
//! This module analyzes the audio output of the NES and applies a screen shake
//! effect to the visual framebuffer when the audio volume exceeds a certain threshold.

#[cfg(feature = "nova")]
use crate::constants::FRAME_RGBA_BYTES;

#[cfg(feature = "nova")]
/// Analyzes audio and applies visual screen shake to a framebuffer.
pub struct ScreenShake {
    state: u32,
    temp_buffer: Vec<u8>,
}

#[cfg(feature = "nova")]
impl Default for ScreenShake {
    fn default() -> Self {
        Self::new(1337)
    }
}

#[cfg(feature = "nova")]
impl ScreenShake {
    /// Creates a new `ScreenShake` with a specific RNG seed.
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            state: if seed == 0 { 0xDEAD_BEEF } else { seed },
            temp_buffer: vec![0; FRAME_RGBA_BYTES],
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.state
    }

    /// Processes an audio chunk and applies a screen shake effect to the framebuffer
    /// if the volume exceeds `threshold`. `intensity` defines the max pixel shift.
    pub fn process(
        &mut self,
        audio: &[i16],
        framebuffer: &mut [u8],
        threshold: i16,
        intensity: usize,
    ) {
        if intensity == 0 || audio.is_empty() {
            return;
        }

        let mut sum = 0u64;
        for &sample in audio {
            sum += sample.unsigned_abs() as u64;
        }
        let avg_volume = (sum / audio.len() as u64) as i16;

        if avg_volume >= threshold {
            let mut dx =
                (self.next_u32() as usize % (intensity * 2 + 1)).saturating_sub(intensity) as isize;
            let dy =
                (self.next_u32() as usize % (intensity * 2 + 1)).saturating_sub(intensity) as isize;

            if dx == 0 && dy == 0 {
                dx = 1;
            }

            self.temp_buffer.copy_from_slice(framebuffer);
            framebuffer.fill(0);

            for y in 0..240 {
                for x in 0..256 {
                    let new_x = x as isize + dx;
                    let new_y = y as isize + dy;

                    if (0..256).contains(&new_x) && (0..240).contains(&new_y) {
                        let src_idx = (y * 256 + x) * 4;
                        let dst_idx = (new_y as usize * 256 + new_x as usize) * 4;
                        framebuffer[dst_idx..dst_idx + 4]
                            .copy_from_slice(&self.temp_buffer[src_idx..src_idx + 4]);
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
    fn test_default() {
        let shaker = ScreenShake::default();
        assert_eq!(shaker.state, 1337);
    }

    #[test]
    fn test_new_zero_seed() {
        let shaker = ScreenShake::new(0);
        assert_eq!(shaker.state, 0xDEAD_BEEF);
    }

    #[test]
    fn test_early_returns() {
        let mut shaker = ScreenShake::new(42);
        let audio = vec![5000; 10];
        let mut frame = vec![0; FRAME_RGBA_BYTES];
        frame[0] = 255;

        // Zero intensity
        shaker.process(&audio, &mut frame, 1000, 0);
        assert_eq!(frame[0], 255);

        // Empty audio
        shaker.process(&[], &mut frame, 1000, 5);
        assert_eq!(frame[0], 255);
    }

    #[test]
    fn test_no_shake_when_quiet() {
        let mut shaker = ScreenShake::new(42);
        let audio = vec![100; 1000]; // Quiet
        let mut frame = vec![0; FRAME_RGBA_BYTES];
        frame[0] = 255; // Set first pixel

        shaker.process(&audio, &mut frame, 1000, 5);

        // Pixel should not move
        assert_eq!(frame[0], 255);
    }

    #[test]
    fn test_shake_when_loud() {
        let mut shaker = ScreenShake::new(42);
        let audio = vec![5000; 1000]; // Loud
        let mut frame = vec![0; FRAME_RGBA_BYTES];
        frame[0] = 255; // Set first pixel

        shaker.process(&audio, &mut frame, 1000, 5);

        // Pixel should move
        assert_eq!(frame[0], 0);
        // It should be somewhere else
        assert!(frame.contains(&255));
    }
}
