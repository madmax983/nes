//! Experimental spatial audio processor.
//!
//! This module provides a way to convert mono NES audio into stereo by dynamically
//! panning the sound based on the X-coordinate of an on-screen entity (like Sprite 0).

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
/// A utility for converting mono audio to stereo based on spatial positions.
pub struct SpatialAudioPanner;

#[cfg(feature = "nova")]
impl SpatialAudioPanner {
    /// Converts a chunk of mono audio samples into stereo samples, panned based on the given X coordinate.
    ///
    /// `target_x` is expected to be an NES screen coordinate (0..=255).
    /// Returns a vector of interleaved stereo samples (Left, Right, Left, Right, ...).
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::spatial_audio::SpatialAudioPanner;
    /// let mono = vec![1000, -1000];
    /// // Center panning
    /// let stereo = SpatialAudioPanner::pan(&mono, 128);
    /// assert_eq!(stereo.len(), 4); // 2 samples * 2 channels
    /// ```
    #[must_use]
    pub fn pan(mono_samples: &[i16], target_x: u8) -> Vec<i16> {
        let mut stereo = Vec::with_capacity(mono_samples.len() * 2);

        // Convert target_x (0-255) to a float pan value -1.0 (left) to 1.0 (right)
        // 128 is center (0.0)
        let pan = (target_x as f32 - 128.0) / 128.0;
        let pan = pan.clamp(-1.0, 1.0);

        // Equal power panning law:
        let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
        let left_gain = angle.cos();
        let right_gain = angle.sin();

        for &sample in mono_samples {
            let left = (sample as f32 * left_gain) as i16;
            let right = (sample as f32 * right_gain) as i16;
            stereo.push(left);
            stereo.push(right);
        }

        stereo
    }

    /// Helper to pan audio based on Sprite 0's position in the core.
    ///
    /// If Sprite 0 is hidden (Y >= 240), it defaults to center panning.
    #[must_use]
    pub fn pan_from_sprite_zero(core: &NesCore, mono_samples: &[i16]) -> Vec<i16> {
        // Sprite 0 Y coordinate is at OAM index 0
        let sprite_y = core.ppu_oam_byte(0);
        // Sprite 0 X coordinate is at OAM index 3
        let sprite_x = core.ppu_oam_byte(3);

        let target_x = if sprite_y < 240 {
            sprite_x
        } else {
            128 // Default center
        };

        Self::pan(mono_samples, target_x)
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::Command;

    #[test]
    fn panning_left_puts_more_volume_in_left_channel() {
        let mono = vec![1000];
        let stereo = SpatialAudioPanner::pan(&mono, 0); // Far left
        assert!(stereo[0] > stereo[1]); // Left > Right
        assert!(stereo[0] > 700);
    }

    #[test]
    fn panning_right_puts_more_volume_in_right_channel() {
        let mono = vec![1000];
        let stereo = SpatialAudioPanner::pan(&mono, 255); // Far right
        assert!(stereo[1] > stereo[0]); // Right > Left
        assert!(stereo[1] > 700);
    }

    #[test]
    fn panning_center_distributes_equally() {
        let mono = vec![1000];
        let stereo = SpatialAudioPanner::pan(&mono, 128); // Center
        // Allow tiny floating point rounding difference
        assert!((stereo[0] - stereo[1]).abs() <= 1);
    }

    #[test]
    fn pan_from_sprite_zero_extracts_x_correctly() {
        let mut core = NesCore::new();
        // Setup a sprite at X=50
        let mut dummy_page = [0xff; 256];
        dummy_page[0] = 100; // Y (visible)
        dummy_page[3] = 50; // X (left of center)
        core.load_cpu_bytes(0x0200, &dummy_page);
        core.write_cpu_bus(0x4014, 0x02); // OAM DMA
        for _ in 0..180 {
            let _ = core.execute(Command::StepCpu);
        }

        let mono = vec![1000];
        let stereo = SpatialAudioPanner::pan_from_sprite_zero(&core, &mono);
        // At X=50, it's left of center, so left channel should be louder
        assert!(stereo[0] > stereo[1]);
    }
}
