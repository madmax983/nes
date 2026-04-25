use std::fs;

use nes_core::{
    Command, NesCore,
    tas::{TasMovie, TasRecorder},
};

use crate::{
    actions::ControlAction,
    config::{AiProfileConfig, GameProfileId},
    episode::EpisodeMetadata,
    error::AiError,
    obs::{FrameStack, downsample_grayscale},
    profile::TaskProfile,
    profiles::smb::SmbProfile,
    reward::{RewardBreakdown, RewardFeatures, RewardModel},
    snapshot::{SnapshotBundle, load_snapshot_bundle, sha256_hex},
};

/// Data returned when the environment is advanced by one action step.
#[derive(Debug, Clone)]
pub struct StepOutput<F> {
    /// Observations or features produced by the step.
    pub features: F,
    /// Detailed breakdown of the reward components.
    pub reward: RewardBreakdown,
    /// True if the episode has reached a terminal state (e.g. death or win).
    pub done: bool,
}

/// A bundled snapshot of the visual inputs provided to the neural network.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationSnapshot {
    /// Number of sequential frames in this snapshot.
    pub frame_stack: usize,
    /// Width of each frame in pixels.
    pub width: usize,
    /// Height of each frame in pixels.
    pub height: usize,
    /// Flattened 1D array of pixel data across all frames.
    pub frames: Vec<f32>,
    /// Any additional numerical features extracted from memory.
    pub features: Vec<f32>,
}

/// Output structure used internally during control actions before observation processing.
#[derive(Debug, Clone)]
pub struct ControlStepOutput {
    /// Broken down components of the reward function.
    pub reward: RewardBreakdown,
    /// Whether the terminal conditions of the environment were met.
    pub done: bool,
}

/// Connects an NES emulator to a specific AI control profile, executing frames
/// and evaluating success/failure for the training feedback loop.
///
/// We need this wrapper so the AI doesn't interact directly with raw NES opcodes.
/// It creates a clean boundary: "Here are the buttons to press, and here is how
/// good you did." It maintains the `NesCore`, processes frame stacks, calculates
/// rewards, and ultimately spits out TAS (Tool-Assisted Speedrun) artifacts.
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
    /// Internal constructor used after generic parameter resolution.
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

    /// Returns a shared reference to the underlying emulator core.
    ///
    /// This is useful for inspecting the current internal state of the NES
    /// without advancing the simulation or mutating its state.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::SmbControlEnv, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let env = SmbControlEnv::from_config(config)?;
    /// // Inspect the current state hash of the emulator.
    /// let hash = env.core().state_hash();
    /// println!("State Hash: {}", hash);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    /// Returns a shared reference to the underlying `NesCore`.
    pub fn core(&self) -> &NesCore {
        &self.core
    }

    /// Returns a mutable reference to the underlying emulator core.
    ///
    /// This allows direct manipulation of the emulator's state, such as
    /// injecting custom commands, modifying memory, or forcibly loading state.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::SmbControlEnv, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # use nes_core::Command;
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let mut env = SmbControlEnv::from_config(config)?;
    /// // Directly execute a command on the underlying core.
    /// env.core_mut().execute(Command::StepFrame).unwrap();
    /// # Ok(())
    /// # }
    /// ```
    /// Returns a mutable reference to the underlying `NesCore`.
    pub fn core_mut(&mut self) -> &mut NesCore {
        &mut self.core
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

    /// Exposes the TAS recording constructed during the current episode.
    #[must_use]
    pub fn recorded_movie(&self) -> &nes_core::tas::TasMovie {
        self.recorder.movie()
    }

    /// Constructs the final episode metadata struct by aggregating internal tracking stats.
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
    /// Returns [`AiError`] if the configured snapshot bundle cannot be loaded,
    /// the ROM cannot be read, or the ROM hash does not match the snapshot.
    pub fn from_config(cfg: AiProfileConfig) -> Result<Self, AiError> {
        if cfg.game != GameProfileId::Smb {
            return Err(AiError::Unsupported("smb env requires game = \"smb\""));
        }
        let snapshot = load_verified_snapshot(&cfg)?;
        let profile = SmbProfile::new(cfg);
        Ok(Self::new(profile, snapshot))
    }
}

/// Standard alias for the `ProfileEnv` specialized to Super Mario Bros.
pub type SmbControlEnv = ProfileEnv<SmbProfile>;

/// Type-erased wrapper for concrete environment specializations like SMB.
///
/// In Rust, generic types `ProfileEnv<P>` infect your entire API surface. This enum
/// exists to hide that complexity behind a single enum, making it trivial to pass
/// "some control environment" across thread bounds without dealing with `Box<dyn Any>`
/// trait object headaches.
/// Exposes the specific environment implementations supported by the AI pipeline.
pub enum AnyControlEnv {
    /// Environment tuned for Super Mario Bros using the `SmbProfile`.
    Smb(Box<SmbControlEnv>),
}

impl AnyControlEnv {
    /// Builds the appropriate control environment for the configured game profile.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if config validation, snapshot loading, or ROM validation fails.
    pub fn from_config(cfg: AiProfileConfig) -> Result<Self, AiError> {
        match cfg.game {
            GameProfileId::Smb => Ok(Self::Smb(Box::new(SmbControlEnv::from_config(cfg)?))),
        }
    }

    /// Resets the current control environment and seeds its observation state.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if the concrete environment cannot restore its snapshot.
    pub fn reset(&mut self) -> Result<(), AiError> {
        match self {
            Self::Smb(env) => {
                env.reset()?;
                Ok(())
            }
        }
    }

    /// Applies one control action and returns reward/done state independent of game-specific features.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if the concrete environment cannot step.
    pub fn step(&mut self, action: ControlAction) -> Result<ControlStepOutput, AiError> {
        match self {
            Self::Smb(env) => {
                let step = env.step(action)?;
                Ok(ControlStepOutput {
                    reward: step.reward,
                    done: step.done,
                })
            }
        }
    }

    /// Returns the current observation for the active control environment.
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if called before reset.
    pub fn observation(&self) -> Result<ObservationSnapshot, AiError> {
        match self {
            Self::Smb(env) => env.observation(),
        }
    }

    /// Retrieves a reference to the active TAS recording.
    #[must_use]
    pub fn recorded_movie(&self) -> &TasMovie {
        match self {
            Self::Smb(env) => env.recorded_movie(),
        }
    }

    /// Constructs the final episode metadata struct by aggregating internal tracking stats.
    #[must_use]
    pub fn finish_episode(&self, total_reward: f32) -> EpisodeMetadata {
        match self {
            Self::Smb(env) => env.finish_episode(total_reward),
        }
    }
}

fn load_verified_snapshot(cfg: &AiProfileConfig) -> Result<SnapshotBundle, AiError> {
    cfg.validate()?;
    let snapshot = load_snapshot_bundle(&cfg.snapshot_path)?;
    let rom_hash = read_rom_hash(&cfg.rom_path)?;
    if snapshot.rom_hash != rom_hash {
        return Err(AiError::RomHashMismatch {
            expected: snapshot.rom_hash,
            found: rom_hash,
        });
    }
    Ok(snapshot)
}

fn read_rom_hash(path: &std::path::Path) -> Result<String, AiError> {
    let rom = fs::read(path).map_err(|source| AiError::RomRead {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(sha256_hex(&rom))
}
