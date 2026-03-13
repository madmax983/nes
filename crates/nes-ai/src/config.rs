use std::path::PathBuf;

use serde::{Deserialize, Deserializer, de::Error as _};

use crate::error::AiError;

pub const MIN_OBSERVATION_DIM: usize = 20;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiProfileConfig {
    pub id: String,
    pub rom_path: PathBuf,
    pub snapshot_path: PathBuf,
    pub bootstrap_tas_path: PathBuf,
    #[serde(deserialize_with = "deserialize_positive_usize")]
    pub frame_stack: usize,
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub frame_skip: u32,
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub max_episode_frames: u32,
    pub observation: ObservationConfig,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationConfig {
    #[serde(deserialize_with = "deserialize_observation_dim")]
    pub width: usize,
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

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardConfig {
    pub forward_progress: f32,
    pub alive_bonus: f32,
    pub stall_penalty: f32,
    pub death_penalty: f32,
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
