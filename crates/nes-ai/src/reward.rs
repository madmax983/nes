use crate::{config::RewardConfig, profiles::smb::SmbFeatures};

pub trait RewardFeatures {
    fn level_progress(&self) -> f32;
    fn player_state(&self) -> u8;
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

#[derive(Debug, Clone, PartialEq)]
pub struct RewardBreakdown {
    pub total: f32,
    pub progress_delta: f32,
    pub alive_bonus: f32,
    pub stall_penalty: f32,
    pub death_penalty: f32,
    pub done: bool,
}

#[derive(Debug, Clone)]
pub struct RewardModel {
    cfg: RewardConfig,
}

impl RewardModel {
    #[must_use]
    pub fn new(cfg: RewardConfig) -> Self {
        Self { cfg }
    }

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
