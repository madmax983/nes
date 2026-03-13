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
            stalled_frames: 0,
            last_features: None,
        }
    }

    pub fn reset(&mut self) -> Result<P::Features, AiError> {
        self.core.load_state(&self.snapshot.snapshot);
        self.recorder = TasRecorder::new();
        self.recorder.start();
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

    pub fn step(&mut self, action: ControlAction) -> Result<StepOutput<P::Features>, AiError> {
        let prev = self
            .last_features
            .clone()
            .ok_or(AiError::Unsupported("step before reset"))?;
        let controller1_bits = action.controller1_bits();
        self.core
            .execute(Command::SetControllerState(controller1_bits))
            .map_err(|_| AiError::Unsupported("step controller"))?;

        for _ in 0..self.profile.config().frame_skip {
            self.recorder.record_frame(controller1_bits);
            self.core
                .execute(Command::StepFrame)
                .map_err(|_| AiError::Unsupported("step frame"))?;
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
        let reward = self.reward.score(&prev, &next, self.stalled_frames);
        self.last_features = Some(next.clone());

        Ok(StepOutput {
            done: reward.done,
            reward,
            features: next,
        })
    }

    #[must_use]
    pub fn core(&self) -> &NesCore {
        &self.core
    }

    #[must_use]
    pub fn finish_episode(&mut self, total_reward: f32) -> EpisodeMetadata {
        EpisodeMetadata {
            profile_id: self.profile.config().id.clone(),
            snapshot_id: self.snapshot.snapshot_id.clone(),
            rom_hash: self.snapshot.rom_hash.clone(),
            total_reward,
            episode_frames: self.recorder.movie().total_frames(),
            final_state_hash: self.core.state_hash(),
        }
    }
}

impl ProfileEnv<SmbProfile> {
    pub fn from_config(cfg: AiProfileConfig) -> Result<Self, AiError> {
        let snapshot = load_snapshot_bundle(&cfg.snapshot_path)?;
        let profile = SmbProfile::new(cfg);
        Ok(Self::new(profile, snapshot))
    }
}

pub type SmbControlEnv = ProfileEnv<SmbProfile>;
