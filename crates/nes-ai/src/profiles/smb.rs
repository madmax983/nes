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
            horizontal_speed: f32::from(core.read_memory(0x0057) as i8),
            vertical_speed: f32::from(core.read_memory(0x009F) as i8),
            airborne: core.read_memory(0x001D) != 0,
            player_state: core.read_memory(0x000E),
            lives: core.read_memory(0x075A),
        }
    }
}
