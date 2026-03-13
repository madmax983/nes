use std::path::PathBuf;

use serde::{Deserialize, Deserializer, de::Error as _};

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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationConfig {
    #[serde(deserialize_with = "deserialize_positive_usize")]
    pub width: usize,
    #[serde(deserialize_with = "deserialize_positive_usize")]
    pub height: usize,
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
