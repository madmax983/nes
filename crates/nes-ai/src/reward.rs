use crate::{config::RewardConfig, profiles::smb::SmbFeatures};

/// Specifies common indicators required for computing scalar AI rewards.
///
/// AI models operate on "carrots" and "sticks." This trait extracts the metrics
/// (like `level_progress`, player `lives`) we need to compute the scalar reward
/// at the end of every step.
pub trait RewardFeatures {
    /// Returns a scalar value indicating the player's progress through the current level.
    fn level_progress(&self) -> f32;
    /// Returns the player's state (e.g. alive, dying, dead).
    fn player_state(&self) -> u8;
    /// Returns the number of lives the player has remaining.
    fn lives(&self) -> u8;
}

impl RewardFeatures for SmbFeatures {
    fn level_progress(&self) -> f32 {
        self.level_progress
    }

    fn player_state(&self) -> u8 {
        self.player_state
    }

    fn lives(&self) -> u8 {
        self.lives
    }
}

/// Breakdown of the reward calculation for a single step.
#[derive(Debug, Clone, PartialEq)]
pub struct RewardBreakdown {
    /// The total computed reward for this step.
    pub total: f32,
    /// The reward or penalty associated with level progress.
    pub progress_delta: f32,
    /// The bonus reward for simply remaining alive during this step.
    pub alive_bonus: f32,
    /// The penalty applied if the player has stalled for too many frames.
    pub stall_penalty: f32,
    /// The penalty applied if the player died during this step.
    pub death_penalty: f32,
    /// Whether the episode should be considered terminal (e.g. death).
    pub done: bool,
}

/// A model for computing rewards during RL training.
///
/// The `RewardModel` takes in a [`RewardConfig`] and is responsible for calculating
/// the scalar reward value given the `prev` and `next` state features, along with
/// any frame stalling information.
///
/// ## Examples
/// ```
/// use nes_ai::reward::RewardModel;
/// use nes_ai::config::RewardConfig;
///
/// let cfg = RewardConfig {
///     forward_progress: 1.0,
///     alive_bonus: 0.1,
///     stall_frames: 600,
///     stall_penalty: -1.0,
///     death_penalty: -5.0,
/// };
/// let model = RewardModel::new(cfg);
/// ```
#[derive(Debug, Clone)]
pub struct RewardModel {
    cfg: RewardConfig,
}

impl RewardModel {
    /// Creates a new `RewardModel` with the given configuration.
    #[must_use]
    pub fn new(cfg: RewardConfig) -> Self {
        Self { cfg }
    }

    /// Computes the reward for a single simulation step.
    #[must_use]
    pub fn score<F: RewardFeatures>(
        &self,
        prev: &F,
        next: &F,
        stalled_frames: u32,
    ) -> RewardBreakdown {
        let done = matches!(next.player_state(), 0x06 | 0x0B) || next.lives() < prev.lives();
        let progress_delta = if done {
            0.0
        } else {
            (next.level_progress() - prev.level_progress()) * self.cfg.forward_progress
        };
        let alive_bonus = if done { 0.0 } else { self.cfg.alive_bonus };
        let stall_penalty = if stalled_frames >= self.cfg.stall_frames {
            self.cfg.stall_penalty
        } else {
            0.0
        };
        let death_penalty = if done { self.cfg.death_penalty } else { 0.0 };

        RewardBreakdown {
            total: progress_delta + alive_bonus + stall_penalty + death_penalty,
            progress_delta,
            alive_bonus,
            stall_penalty,
            death_penalty,
            done,
        }
    }
}
