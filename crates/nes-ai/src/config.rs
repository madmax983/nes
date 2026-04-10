//! Configuration structs for AI profiles and environment constraints.
//!
//! Defines the settings required to initialize environments, process observations,
//! calculate rewards, and specify game ROMs/snapshots.

use std::path::PathBuf;

use serde::{Deserialize, Deserializer, de::Error as _};

use crate::error::AiError;

/// The minimum allowed dimension (width or height) for visual observations.
pub const MIN_OBSERVATION_DIM: usize = 20;

/// Identifier for supported game profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GameProfileId {
    /// Super Mario Bros. (NES)
    #[default]
    Smb,
}

/// The root configuration defining an AI training or evaluation profile.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiProfileConfig {
    /// The target game profile identifier.
    #[serde(default)]
    pub game: GameProfileId,
    /// A unique human-readable ID for this profile instance.
    pub id: String,
    /// Path to the game ROM file.
    pub rom_path: PathBuf,
    /// Path to the initial state snapshot bundle.
    pub snapshot_path: PathBuf,
    /// Path to an optional TAS movie to bootstrap state.
    pub bootstrap_tas_path: PathBuf,
    /// Number of consecutive frames to stack for temporal observations.
    #[serde(deserialize_with = "deserialize_positive_usize")]
    pub frame_stack: usize,
    /// Number of frames to advance the emulator per agent action.
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub frame_skip: u32,
    /// Maximum number of frames allowed per episode before forcing termination.
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub max_episode_frames: u32,
    /// Configuration for downsampling and processing visual observations.
    pub observation: ObservationConfig,
    /// Weights and penalties used for calculating episode rewards.
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

/// Settings for resizing and processing the emulator framebuffer.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationConfig {
    /// Target width in pixels for downsampled frames.
    #[serde(deserialize_with = "deserialize_observation_dim")]
    pub width: usize,
    /// Target height in pixels for downsampled frames.
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

/// Weights and coefficients used to calculate the reward signal.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardConfig {
    /// Multiplier applied to rightward x-axis progression.
    pub forward_progress: f32,
    /// Flat bonus awarded for every frame survived.
    pub alive_bonus: f32,
    /// Penalty applied when the agent fails to make progress.
    pub stall_penalty: f32,
    /// Penalty applied when the agent dies.
    pub death_penalty: f32,
    /// Number of consecutive frames without progress required to trigger a stall penalty.
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
