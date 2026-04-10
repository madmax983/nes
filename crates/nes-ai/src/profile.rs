use nes_core::NesCore;

use crate::config::AiProfileConfig;

/// Defines the necessary operations for extracting and evaluating features
/// from the emulator state to power AI training loops.
///
/// We can't just feed raw 64KB memory dumps to our AI model. It would never converge.
/// By implementing `TaskProfile`, you provide a "lens" for the AI, allowing it to
/// see only the specific variables that matter for the task (like player X/Y coords).
pub trait TaskProfile {
    /// The concrete type representing game-specific memory values.
    type Features: Clone + PartialEq + core::fmt::Debug;

    /// Returns the configuration associated with this profile instance.
    fn config(&self) -> &AiProfileConfig;

    /// Parses the raw emulator memory into structured feature values.
    fn decode_features(&self, core: &NesCore) -> Self::Features;

    /// Normalizes and flattens structured features into a `f32` tensor format.
    fn encode_features(&self, features: &Self::Features) -> Vec<f32>;

    /// Returns the exact number of normalized floats output by `encode_features`.
    fn feature_count(&self) -> usize;
}
