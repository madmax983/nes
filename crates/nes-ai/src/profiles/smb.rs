use nes_core::NesCore;

use crate::{config::AiProfileConfig, profile::TaskProfile};

#[derive(Debug, Clone, PartialEq)]
pub struct SmbFeatures {
    pub level_progress: f32,
    pub horizontal_speed: f32,
    pub vertical_speed: f32,
    pub airborne: bool,
    pub player_state: u8,
    pub lives: u8,
}

#[derive(Debug, Clone)]
pub struct SmbProfile {
    config: AiProfileConfig,
}

impl SmbProfile {
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
