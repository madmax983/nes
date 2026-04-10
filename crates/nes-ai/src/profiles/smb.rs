//! Profile for Super Mario Bros. (NES).
//!
//! This module teaches the AI how to interpret the memory structure of
//! Super Mario Bros. It maps raw hexadecimal RAM addresses into meaningful
//! state vectors (like Mario's speed, lives, and level progress).

use nes_core::NesCore;

use crate::{config::AiProfileConfig, profile::TaskProfile};

/// Game-specific features extracted from Super Mario Bros RAM.
#[derive(Debug, Clone, PartialEq)]
pub struct SmbFeatures {
    /// Mario's total horizontal progress through the level (pixels + page offsets).
    pub level_progress: f32,
    /// Mario's current horizontal velocity.
    pub horizontal_speed: f32,
    /// Mario's current vertical velocity.
    pub vertical_speed: f32,
    /// Indicates whether Mario is jumping or falling.
    pub airborne: bool,
    /// Mario's current state (e.g. normal, dying, power-up animation).
    pub player_state: u8,
    /// Total remaining lives.
    pub lives: u8,
}

/// The implementation of [`TaskProfile`] specific to Super Mario Bros.
#[derive(Debug, Clone)]
pub struct SmbProfile {
    config: AiProfileConfig,
}

impl SmbProfile {
    /// Creates a new profile using the provided configuration.
    #[must_use]
    pub fn new(config: AiProfileConfig) -> Self {
        Self { config }
    }
}

impl TaskProfile for SmbProfile {
    type Features = SmbFeatures;

    fn config(&self) -> &AiProfileConfig {
        &self.config
    }

    fn decode_features(&self, core: &NesCore) -> Self::Features {
        SmbFeatures {
            level_progress: f32::from(core.read_memory(0x006D)) * 256.0
                + f32::from(core.read_memory(0x0086)),
            horizontal_speed: f32::from(decode_signed_byte(core.read_memory(0x0057))),
            vertical_speed: f32::from(decode_signed_byte(core.read_memory(0x009F))),
            airborne: core.read_memory(0x001D) != 0,
            player_state: core.read_memory(0x000E),
            lives: core.read_memory(0x075A),
        }
    }

    fn encode_features(&self, features: &Self::Features) -> Vec<f32> {
        vec![
            features.level_progress / 4_096.0,
            features.horizontal_speed / 16.0,
            features.vertical_speed / 16.0,
            if features.airborne { 1.0 } else { 0.0 },
            f32::from(features.player_state) / 16.0,
            f32::from(features.lives) / 10.0,
        ]
    }

    fn feature_count(&self) -> usize {
        6
    }
}

fn decode_signed_byte(value: u8) -> i16 {
    if value < 0x80 {
        i16::from(value)
    } else {
        i16::from(value) - 0x100
    }
}

#[cfg(test)]
mod tests {
    use super::decode_signed_byte;

    #[test]
    fn decode_signed_byte_handles_twos_complement_values() {
        assert_eq!(decode_signed_byte(0x00), 0);
        assert_eq!(decode_signed_byte(0x7F), 127);
        assert_eq!(decode_signed_byte(0x80), -128);
        assert_eq!(decode_signed_byte(0xFF), -1);
    }
}
