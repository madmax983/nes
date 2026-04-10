//! Reward calculation and state tracking for RL agents.
//!
//! This module translates raw game memory features into numeric reward
//! signals that guide the training of the reinforcement learning agent.

use crate::{config::RewardConfig, profiles::smb::SmbFeatures};

/// Specifies common indicators required for computing scalar AI rewards.
///
/// AI models operate on "carrots" and "sticks." This trait extracts the metrics
/// (like `level_progress`, player `lives`) we need to compute the scalar reward
/// at the end of every step.
pub trait RewardFeatures {
    /// Returns a numeric value indicating forward progression through the level.
    fn level_progress(&self) -> f32;
    /// Returns a game-specific byte representing the player's current status (e.g. dying).
    fn player_state(&self) -> u8;
    /// Returns the number of lives the player currently has remaining.
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

/// A detailed accounting of exactly how a reward value was calculated.
#[derive(Debug, Clone, PartialEq)]
pub struct RewardBreakdown {
    /// The final combined scalar reward for this step.
    pub total: f32,
    /// Reward generated directly from moving to the right.
    pub progress_delta: f32,
    /// Flat bonus awarded for surviving this frame.
    pub alive_bonus: f32,
    /// Penalty applied for failing to make forward progress.
    pub stall_penalty: f32,
    /// Penalty applied because the agent died on this step.
    pub death_penalty: f32,
    /// True if the episode should terminate due to death or reaching a goal.
    pub done: bool,
}

/// Calculates reward signals by comparing consecutive state features.
#[derive(Debug, Clone)]
pub struct RewardModel {
    cfg: RewardConfig,
}

impl RewardModel {
    /// Creates a new reward model configured with specific weights and penalties.
    #[must_use]
    pub fn new(cfg: RewardConfig) -> Self {
        Self { cfg }
    }

    /// Evaluates the transition between two states and returns a scored reward.
    ///
    /// This method looks at the `prev` features and `next` features, applies the
    /// weights from the associated [`RewardConfig`], and checks if the episode
    /// has terminated (e.g. by losing a life).
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
