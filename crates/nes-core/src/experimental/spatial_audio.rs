//! Experimental spatial audio generator.
//!
//! This module mashes up the emulator's Mono Audio output with Object Attribute Memory (OAM)
//! spatial queries to create a pseudo-stereo "spatial audio" effect. By tracking a specific
//! sprite (like Sprite 0, which is often the player character or a main projectile) on the X-axis,
//! it pans the audio left or right based on the character's screen position!

#[cfg(feature = "nova")]
use crate::NesCore;

#[cfg(feature = "nova")]
use crate::experimental::oam_spatial_query::OamSpatialQuery;

#[cfg(feature = "nova")]
/// A spatial audio processor that creates a stereo field from mono audio based on sprite position.
///
/// `SpatialAudioPanner` tracks a specific sprite in the NES OAM (Object Attribute Memory) and
/// adjusts the volume of the left and right audio channels accordingly. When the tracked sprite
/// moves to the left side of the screen, the audio pans left, and vice versa.
pub struct SpatialAudioPanner {
    /// The index of the sprite to track in OAM (0..64).
    pub target_sprite_index: u8,
}

#[cfg(feature = "nova")]
impl SpatialAudioPanner {
    /// Creates a new `SpatialAudioPanner` tracking the specified sprite index.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::experimental::spatial_audio::SpatialAudioPanner;
    /// let panner = SpatialAudioPanner::new(0); // Track Sprite 0 (often the player)
    /// ```
    #[must_use]
    pub fn new(target_sprite_index: u8) -> Self {
        Self {
            target_sprite_index,
        }
    }

    /// Processes a chunk of mono audio samples and returns an interleaved stereo chunk (L, R, L, R)
    /// where the panning is determined by the `target_sprite_index`'s X coordinate on the screen.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use nes_core::NesCore;
    /// # use nes_core::experimental::spatial_audio::SpatialAudioPanner;
    /// let core = NesCore::new();
    /// let panner = SpatialAudioPanner::new(0);
    ///
    /// // Generate some mono silence/audio
    /// let mono_samples = vec![1000, 2000, -1000, -2000];
    ///
    /// // Process into stereo
    /// let stereo_samples = panner.process_stereo(&core, &mono_samples);
    /// assert_eq!(stereo_samples.len(), mono_samples.len() * 2);
    /// ```
    #[must_use]
    pub fn process_stereo(&self, core: &NesCore, mono_samples: &[i16]) -> Vec<i16> {
        let oam = OamSpatialQuery::new(core);
        let sprites = oam.sprites();

        let mut x_pos = 128; // Default to center if sprite not found or bounds check fails
        if let Some(sprite) = sprites.iter().find(|s| s.index == self.target_sprite_index) {
            x_pos = sprite.x;
        }

        // x_pos is 0..=255. We want left volume to be higher when x is close to 0,
        // and right volume higher when x is close to 255.
        // We use a simple linear panning model.
        let left_vol = (255 - x_pos as u32) as f32 / 255.0;
        let right_vol = (x_pos as u32) as f32 / 255.0;

        let mut stereo = Vec::with_capacity(mono_samples.len() * 2);
        for &sample in mono_samples {
            let left_sample = (sample as f32 * left_vol) as i16;
            let right_sample = (sample as f32 * right_vol) as i16;
            stereo.push(left_sample);
            stereo.push(right_sample);
        }

        stereo
    }
}

#[cfg(all(test, feature = "nova"))]
mod tests {
    use super::*;
    use crate::NesCore;

    #[test]
    fn test_spatial_audio_panner() {
        let mut core = NesCore::new();
        let panner = SpatialAudioPanner::new(0);

        // Put sprite 0 at the far left (x=0)
        let mut dummy_page = [0xff; 256];
        dummy_page[3] = 0; // Sprite 0 X
        core.load_cpu_bytes(0x0200, &dummy_page);
        core.write_cpu_bus(0x4014, 0x02); // Trigger OAM DMA
        for _ in 0..600 {
            let _ = core.execute(crate::Command::StepCpu);
        }

        let mono = vec![1000];
        let stereo = panner.process_stereo(&core, &mono);

        // At x=0, left should be full volume, right should be 0
        assert_eq!(stereo[0], 1000); // Left
        assert_eq!(stereo[1], 0); // Right

        // Put sprite 0 at the far right (x=255)
        let mut dummy_page2 = [0xff; 256];
        dummy_page2[3] = 255; // Sprite 0 X
        core.load_cpu_bytes(0x0200, &dummy_page2);
        core.write_cpu_bus(0x4014, 0x02); // Trigger OAM DMA
        for _ in 0..600 {
            let _ = core.execute(crate::Command::StepCpu);
        }

        let stereo_right = panner.process_stereo(&core, &mono);
        assert_eq!(stereo_right[0], 0); // Left
        assert_eq!(stereo_right[1], 1000); // Right

        // Put sprite 0 at the center (x=127)
        let mut dummy_page3 = [0xff; 256];
        dummy_page3[3] = 127; // Sprite 0 X
        core.load_cpu_bytes(0x0200, &dummy_page3);
        core.write_cpu_bus(0x4014, 0x02); // Trigger OAM DMA
        for _ in 0..600 {
            let _ = core.execute(crate::Command::StepCpu);
        }

        let stereo_center = panner.process_stereo(&core, &mono);
        // ~50% volume for both
        assert!(stereo_center[0] > 400 && stereo_center[0] < 600);
        assert!(stereo_center[1] > 400 && stereo_center[1] < 600);
    }
}
