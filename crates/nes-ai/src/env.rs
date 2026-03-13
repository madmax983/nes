use nes_core::{Command, NesCore, tas::TasRecorder};

use crate::{
    actions::ControlAction,
    config::AiProfileConfig,
    episode::EpisodeMetadata,
    error::AiError,
    obs::{FrameStack, downsample_grayscale},
    profile::TaskProfile,
    profiles::smb::SmbProfile,
    reward::{RewardBreakdown, RewardFeatures, RewardModel},
    snapshot::{SnapshotBundle, load_snapshot_bundle},
};

#[derive(Debug, Clone)]
pub struct StepOutput<F> {
    pub features: F,
    pub reward: RewardBreakdown,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservationSnapshot {
    pub frame_stack: usize,
    pub width: usize,
    pub height: usize,
    pub frames: Vec<f32>,
    pub features: Vec<f32>,
}

pub struct ProfileEnv<P>
where
    P: TaskProfile,
    P::Features: RewardFeatures,
{
    core: NesCore,
    profile: P,
    snapshot: SnapshotBundle,
    frame_stack: FrameStack,
    reward: RewardModel,
    recorder: TasRecorder,
    episode_frames: u32,
    episode_done: bool,
    stalled_frames: u32,
    last_features: Option<P::Features>,
}

impl<P> ProfileEnv<P>
where
    P: TaskProfile,
    P::Features: RewardFeatures,
{
    #[must_use]
    pub fn new(profile: P, snapshot: SnapshotBundle) -> Self {
        let cfg = profile.config().clone();
        Self {
            core: NesCore::new(),
            profile,
            snapshot,
            frame_stack: FrameStack::new(
                cfg.frame_stack,
                cfg.observation.width * cfg.observation.height,
            ),
            reward: RewardModel::new(cfg.reward),
            recorder: TasRecorder::new(),
            episode_frames: 0,
            episode_done: false,
            stalled_frames: 0,
            last_features: None,
        }
    }

    /// Restores the configured snapshot and seeds the frame stack for a new episode.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if the emulator state cannot be restored.
    pub fn reset(&mut self) -> Result<P::Features, AiError> {
        self.core.load_state(&self.snapshot.snapshot);
        self.recorder = TasRecorder::new();
        self.recorder.start();
        self.episode_frames = 0;
        self.episode_done = false;
        self.stalled_frames = 0;
        self.frame_stack = FrameStack::new(
            self.profile.config().frame_stack,
            self.profile.config().observation.width * self.profile.config().observation.height,
        );

        let features = self.profile.decode_features(&self.core);
        self.last_features = Some(features.clone());

        let frame = downsample_grayscale(
            &self.core.framebuffer_rgba(),
            self.profile.config().observation.width,
            self.profile.config().observation.height,
        );
        for _ in 0..self.profile.config().frame_stack {
            self.frame_stack.push(frame.clone());
        }

        Ok(features)
    }

    /// Applies one discrete action and advances the emulator by the configured frame skip.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if called before [`Self::reset`] or if controller/frame
    /// commands cannot be executed by the emulator core.
    pub fn step(&mut self, action: ControlAction) -> Result<StepOutput<P::Features>, AiError> {
        if self.episode_done {
            return Err(AiError::Unsupported("step after episode done"));
        }
        let prev = self
            .last_features
            .clone()
            .ok_or(AiError::Unsupported("step before reset"))?;
        let controller1_bits = action.controller1_bits();
        self.core
            .execute(Command::SetControllerState(controller1_bits))
            .map_err(|_| AiError::Unsupported("step controller"))?;

        let mut budget_done = false;
        for _ in 0..self.profile.config().frame_skip {
            if self.episode_frames >= self.profile.config().max_episode_frames {
                budget_done = true;
                break;
            }
            self.recorder.record_frame(controller1_bits);
            self.core
                .execute(Command::StepFrame)
                .map_err(|_| AiError::Unsupported("step frame"))?;
            self.episode_frames = self.episode_frames.saturating_add(1);
        }

        let frame = downsample_grayscale(
            &self.core.framebuffer_rgba(),
            self.profile.config().observation.width,
            self.profile.config().observation.height,
        );
        self.frame_stack.push(frame);

        let next = self.profile.decode_features(&self.core);
        self.stalled_frames = if next.level_progress() > prev.level_progress() {
            0
        } else {
            self.stalled_frames
                .saturating_add(self.profile.config().frame_skip)
        };
        let mut reward = self.reward.score(&prev, &next, self.stalled_frames);
        reward.done = reward.done
            || budget_done
            || self.episode_frames >= self.profile.config().max_episode_frames;
        self.last_features = Some(next.clone());
        self.episode_done = reward.done;

        Ok(StepOutput {
            done: self.episode_done,
            reward,
            features: next,
        })
    }

    #[must_use]
    pub fn core(&self) -> &NesCore {
        &self.core
    }

    /// Returns the current observation as flattened stacked frames and encoded numeric features.
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Unsupported`] if called before [`Self::reset`].
    pub fn observation(&self) -> Result<ObservationSnapshot, AiError> {
        let features = self
            .last_features
            .as_ref()
            .ok_or(AiError::Unsupported("observation before reset"))?;

        Ok(ObservationSnapshot {
            frame_stack: self.profile.config().frame_stack,
            width: self.profile.config().observation.width,
            height: self.profile.config().observation.height,
            frames: self.frame_stack.flattened(),
            features: self.profile.encode_features(features),
        })
    }

    #[must_use]
    pub fn recorded_movie(&self) -> &nes_core::tas::TasMovie {
        self.recorder.movie()
    }

    #[must_use]
    pub fn finish_episode(&self, total_reward: f32) -> EpisodeMetadata {
        EpisodeMetadata {
            profile_id: self.profile.config().id.clone(),
            snapshot_id: self.snapshot.snapshot_id.clone(),
            rom_hash: self.snapshot.rom_hash.clone(),
            total_reward,
            episode_frames: u64::from(self.episode_frames),
            final_state_hash: self.core.state_hash(),
        }
    }
}

impl ProfileEnv<SmbProfile> {
    /// Builds an SMB control environment from a validated AI profile config.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if the configured snapshot bundle cannot be loaded.
    pub fn from_config(cfg: AiProfileConfig) -> Result<Self, AiError> {
        let snapshot = load_snapshot_bundle(&cfg.snapshot_path)?;
        let profile = SmbProfile::new(cfg);
        Ok(Self::new(profile, snapshot))
    }
}

pub type SmbControlEnv = ProfileEnv<SmbProfile>;
