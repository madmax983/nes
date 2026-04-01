use nes_core::NesCore;

use crate::config::AiProfileConfig;

/// Defines the necessary operations for extracting and evaluating features
/// from the emulator state to power AI training loops.
///
/// We can't just feed raw 64KB memory dumps to our AI model. It would never converge.
/// By implementing `TaskProfile`, you provide a "lens" for the AI, allowing it to
/// see only the specific variables that matter for the task (like player X/Y coords).
pub trait TaskProfile {
    type Features: Clone + PartialEq + core::fmt::Debug;

    fn config(&self) -> &AiProfileConfig;
    fn decode_features(&self, core: &NesCore) -> Self::Features;
    fn encode_features(&self, features: &Self::Features) -> Vec<f32>;
    fn feature_count(&self) -> usize;
}
