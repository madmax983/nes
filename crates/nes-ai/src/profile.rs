use nes_core::NesCore;

use crate::config::AiProfileConfig;

pub trait TaskProfile {
    type Features: Clone + PartialEq + core::fmt::Debug;

    fn config(&self) -> &AiProfileConfig;
    fn decode_features(&self, core: &NesCore) -> Self::Features;
}
