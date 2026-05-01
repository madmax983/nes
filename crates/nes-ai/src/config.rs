//! The Architect's Blueprints: AI Profile Configuration
//!
//! An AI doesn't just "play a game." It needs to know how fast time moves, what the world looks like,
//! and exactly how many points it loses for jumping into a bottomless pit.
//!
//! This module defines the rigid structure of those rules. These structs map directly to the
//! user-provided TOML or JSON configurations, defining the hyperparameters, filesystem paths,
//! and behavioral rules that govern the agent's existence.

use std::path::PathBuf;

use serde::{Deserialize, Deserializer, de::Error as _};

use crate::error::AiError;

/// The minimum allowed dimension (width or height) in pixels for an AI observation space.
pub const MIN_OBSERVATION_DIM: usize = 20;

/// Identifies the specific NES game being played by the AI profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GameProfileId {
    /// Super Mario Bros. (1985)
    #[default]
    Smb,
}

/// The master ledger dictating the terms of an agent's existence.
///
/// An `AiProfileConfig` isn't just settings; it's the physics engine of the agent's mind.
/// It decides how fast the agent perceives time (`frame_skip`), how deep its memory is (`frame_stack`),
/// and what actions bring it joy or pain (`reward`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiProfileConfig {
    /// The specific universe the agent will inhabit.
    #[serde(default)]
    pub game: GameProfileId,
    /// A unique name for this training run, used for cataloging checkpoints and artifacts.
    pub id: String,
    /// Where to find the cartridge (ROM) for the game.
    pub rom_path: PathBuf,
    /// Where to find the exact moment in time (`.state.json`) the agent will wake up in.
    pub snapshot_path: PathBuf,
    /// An optional seed script of inputs to fast-forward the agent through menus.
    pub bootstrap_tas_path: PathBuf,
    /// How many sequential glimpses of the world the agent holds in its mind at once.
    /// Critical for allowing a neural network to infer momentum and direction.
    #[serde(deserialize_with = "deserialize_positive_usize")]
    pub frame_stack: usize,
    /// How many frames the emulator fast-forwards between each agent decision.
    /// High skip means fast training but poor reaction time.
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub frame_skip: u32,
    /// The absolute time limit before the universe collapses and the episode resets.
    #[serde(deserialize_with = "deserialize_positive_u32")]
    pub max_episode_frames: u32,
    /// Rules for crushing the beautiful NES display down into cold, mathematical tensors.
    pub observation: ObservationConfig,
    /// The psychological carrot and stick. Defines what the agent values.
    pub reward: RewardConfig,
}

impl AiProfileConfig {
    /// Validates the delicate runtime invariants of the configuration.
    ///
    /// Deserializing from JSON catches most of these, but if you construct a profile
    /// by hand in code, you must ensure the physics engine doesn't divide by zero.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use nes_ai::config::{AiProfileConfig, GameProfileId, ObservationConfig, RewardConfig};
    /// use std::path::PathBuf;
    ///
    /// let config = AiProfileConfig {
    ///     game: GameProfileId::Smb,
    ///     id: "my-hero".to_string(),
    ///     rom_path: PathBuf::from("dummy.nes"),
    ///     snapshot_path: PathBuf::from("dummy.state.json"),
    ///     bootstrap_tas_path: PathBuf::new(),
    ///     frame_stack: 4,
    ///     frame_skip: 4,
    ///     max_episode_frames: 1000,
    ///     observation: ObservationConfig { width: 84, height: 84 },
    ///     reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.1, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 60 },
    /// };
    ///
    /// // The universe is sound.
    /// assert!(config.validate().is_ok());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if zero-values or dimensions smaller than [`MIN_OBSERVATION_DIM`] are found.
    pub fn validate(&self) -> Result<(), AiError> {
        validate_positive_usize(self.frame_stack, "frame_stack")?;
        validate_positive_u32(self.frame_skip, "frame_skip")?;
        validate_positive_u32(self.max_episode_frames, "max_episode_frames")?;
        self.observation.validate()?;
        self.reward.validate()?;
        Ok(())
    }
}

/// How the agent sees the world.
///
/// An NES renders at 256x240, but training a convolutional neural network on that many
/// pixels is computationally agonizing. This config dictates how brutally the screen
/// is squashed down before the agent sees it.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationConfig {
    /// The target width of the downsampled tensor. Must be at least `MIN_OBSERVATION_DIM`.
    #[serde(deserialize_with = "deserialize_observation_dim")]
    pub width: usize,
    /// The target height of the downsampled tensor. Must be at least `MIN_OBSERVATION_DIM`.
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

/// The psychological framework of the agent.
///
/// This structure defines the literal meaning of life for the AI. Tweak these numbers,
/// and you change whether the agent rushes fearlessly forward or cowers safely in a corner.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardConfig {
    /// The carrot: Points awarded for moving in the right direction.
    pub forward_progress: f32,
    /// The heartbeat: A tiny drip of points given just for staying alive.
    pub alive_bonus: f32,
    /// The stick: Points drained when the agent stands still, forcing it out of local minima.
    pub stall_penalty: f32,
    /// The tragedy: A massive point deduction applied the moment the agent meets its end.
    pub death_penalty: f32,
    /// The patience: How many frames the agent can stand still before the `stall_penalty` kicks in.
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
