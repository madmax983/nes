use std::path::PathBuf;

use serde::{Deserialize, Deserializer, de::Error as _};

use crate::error::AiError;

/// Minimum valid dimension for observation pixel tensors.
/// Prevent configurations from scaling inputs down so far that the network cannot identify objects.
pub const MIN_OBSERVATION_DIM: usize = 20;

/// Identifies a specific NES game targeted for reinforcement learning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GameProfileId {
    /// Super Mario Bros (NTSC version).
    #[default]
    Smb,
}

/// Core parameters governing how the environment is configured and stepped during training.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiProfileConfig {
    /// The target game for this profile.
    #[serde(default)]
    pub game: GameProfileId,
    /// A unique string identifier for this profile configuration.
    pub id: String,
    /// Path to the ROM file required to boot the environment.
    pub rom_path: PathBuf,
    /// Path to a JSON snapshot used to inject the initial memory state.
    pub snapshot_path: PathBuf,
    /// Path to a TAS recording used to step the environment to a specific start state.
    pub bootstrap_tas_path: PathBuf,
    /// Number of consecutive observations to stack together as input.
    #[serde(deserialize_with = "deserialize_positive_usize")]
    pub frame_stack: usize,
    /// Number of emulator frames to advance per action step.
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub frame_skip: u32,
    /// Hard limit on the number of frames allowed before truncating an episode.
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub max_episode_frames: u32,
    /// Configuration for downscaling the visual observation tensor.
    pub observation: ObservationConfig,
    /// Weights and parameters governing the reward signal.
    pub reward: RewardConfig,
}

impl AiProfileConfig {
    /// Validates runtime config invariants for manually constructed profiles.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] when any field violates the same constraints enforced
    /// by deserialization.
    pub fn validate(&self) -> Result<(), AiError> {
        validate_positive_usize(self.frame_stack, "frame_stack")?;
        validate_positive_u32(self.frame_skip, "frame_skip")?;
        validate_positive_u32(self.max_episode_frames, "max_episode_frames")?;
        self.observation.validate()?;
        self.reward.validate()?;
        Ok(())
    }
}

/// Defines how the raw 256x240 NES framebuffer is processed into a model input.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationConfig {
    /// The target width in pixels.
    #[serde(deserialize_with = "deserialize_observation_dim")]
    pub width: usize,
    /// The target height in pixels.
    #[serde(deserialize_with = "deserialize_observation_dim")]
    pub height: usize,
}

impl ObservationConfig {
    fn validate(&self) -> Result<(), AiError> {
        validate_positive_usize(self.width, "observation width")?;
        validate_positive_usize(self.height, "observation height")?;
        validate_min_observation_dim(self.width, "observation width")?;
        validate_min_observation_dim(self.height, "observation height")?;
        Ok(())
    }
}

/// Contains coefficients and thresholds used to shape the reinforcement learning reward.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardConfig {
    /// Scalar reward applied per unit of progress.
    pub forward_progress: f32,
    /// Flat scalar reward applied on every frame the agent survives.
    pub alive_bonus: f32,
    /// Flat scalar penalty applied when the agent triggers the stall condition.
    pub stall_penalty: f32,
    /// Flat scalar penalty applied when the agent dies.
    pub death_penalty: f32,
    /// Consecutive frames with no progress required to trigger the stall penalty.
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub stall_frames: u32,
}

impl RewardConfig {
    fn validate(&self) -> Result<(), AiError> {
        validate_positive_u32(self.stall_frames, "stall_frames")
    }
}

fn deserialize_positive_usize<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let value = usize::deserialize(deserializer)?;
    if value == 0 {
        return Err(D::Error::custom("value must be greater than zero"));
    }
    Ok(value)
}

fn deserialize_observation_dim<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let value = deserialize_positive_usize(deserializer)?;
    if value < MIN_OBSERVATION_DIM {
        return Err(D::Error::custom(format!(
            "value must be at least {MIN_OBSERVATION_DIM} pixels"
        )));
    }
    Ok(value)
}

fn deserialize_positive_u32<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let value = u32::deserialize(deserializer)?;
    if value == 0 {
        return Err(D::Error::custom("value must be greater than zero"));
    }
    Ok(value)
}

fn validate_positive_usize(value: usize, field: &'static str) -> Result<(), AiError> {
    if value == 0 {
        return Err(AiError::Unsupported(match field {
            "frame_stack" => "frame_stack must be greater than zero",
            "observation width" => "observation width must be greater than zero",
            "observation height" => "observation height must be greater than zero",
            _ => "usize config value must be greater than zero",
        }));
    }
    Ok(())
}

fn validate_positive_u32(value: u32, field: &'static str) -> Result<(), AiError> {
    if value == 0 {
        return Err(AiError::Unsupported(match field {
            "frame_skip" => "frame_skip must be greater than zero",
            "max_episode_frames" => "max_episode_frames must be greater than zero",
            "stall_frames" => "stall_frames must be greater than zero",
            _ => "u32 config value must be greater than zero",
        }));
    }
    Ok(())
}

fn validate_min_observation_dim(value: usize, field: &'static str) -> Result<(), AiError> {
    if value < MIN_OBSERVATION_DIM {
        return Err(AiError::Unsupported(match field {
            "observation width" => "observation width must be at least 20 pixels",
            "observation height" => "observation height must be at least 20 pixels",
            _ => "observation dimension must be at least 20 pixels",
        }));
    }
    Ok(())
}
