use nes_core::NesCore;

use crate::config::AiProfileConfig;

/// Defines the necessary operations for extracting and evaluating features
/// from the emulator state to power AI training loops.
///
/// We can't just feed raw 64KB memory dumps to our AI model. It would never converge.
/// By implementing `TaskProfile`, you provide a "lens" for the AI, allowing it to
/// see only the specific variables that matter for the task (like player X/Y coords).
pub trait TaskProfile {
    /// The strongly-typed intermediate representation of the extracted RAM variables.
    type Features: Clone + PartialEq + core::fmt::Debug;

    /// Returns the static training configuration profile for this task.
    fn config(&self) -> &AiProfileConfig;
    /// Inspects the emulator RAM and computes the strongly-typed feature set.
    fn decode_features(&self, core: &NesCore) -> Self::Features;
    /// Transforms the strongly-typed features into a normalized float array for network ingestion.
    fn encode_features(&self, features: &Self::Features) -> Vec<f32>;
    /// Returns the exact length of the vector produced by `encode_features`.
    fn feature_count(&self) -> usize;
}
