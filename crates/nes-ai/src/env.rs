//! The Matrix: Simulated Environments for AI Agents
//!
//! An AI agent does not know what an "NES" is. It only knows that when it provides an
//! action, the universe replies with an observation and a reward.
//!
//! This module wraps the `NesCore` emulator inside a reinforcement learning environment.
//! It handles the dirty work of downsampling frames, managing frame-skipping, computing
//! rewards, and recording TAS movies, so the AI can focus purely on learning to win.

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

/// The universe's reply to an agent's action.
#[derive(Debug, Clone)]
pub struct StepOutput<F> {
    /// The raw, game-specific memory state (e.g., Mario's coordinates, enemy positions).
    pub features: F,
    /// The score calculated by the reward model, broken down by category.
    pub reward: RewardBreakdown,
    /// Indicates if the agent died, won, or ran out of time. If true, the simulation must be reset.
    pub done: bool,
}

/// A frozen glimpse of the world, packaged neatly for a neural network.
///
/// This snapshot flattens history (stacked frames) and reality (game features) into
/// contiguous floating-point buffers, ready to be ingested by Burn tensors.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationSnapshot {
    /// How many sequential glimpses of the world are stacked here.
    pub frame_stack: usize,
    /// The width of a single downsampled frame.
    pub width: usize,
    /// The height of a single downsampled frame.
    pub height: usize,
    /// The visual pixels, crushed into grayscale and flattened.
    pub frames: Vec<f32>,
    /// The numeric state vectors (like positions or speeds), flattened.
    pub features: Vec<f32>,
}

/// A stripped-down reply to an agent's action, useful for agnostic evaluation loops.
#[derive(Debug, Clone)]
pub struct ControlStepOutput {
    /// The score calculated by the reward model.
    pub reward: RewardBreakdown,
    /// Indicates if the simulation has reached a terminal state.
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
    /// Constructs a new, uninitialized sandbox.
    ///
    /// The environment is born in limbo. You must call `reset()` to inject the snapshot
    /// and breathe life into the emulator before asking for observations or taking steps.
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// // Example ignored in tests due to the complexity of constructing a valid CoreSnapshot from scratch.
    /// use nes_ai::{env::ProfileEnv, config::AiProfileConfig, snapshot::load_snapshot_bundle};
    /// use nes_ai::profiles::smb::SmbProfile;
    ///
    /// let config = AiProfileConfig { /* ... */ };
    /// let profile = SmbProfile::new(config.clone());
    /// let snapshot = load_snapshot_bundle(&config.snapshot_path)?;
    ///
    /// // Create the environment
    /// let mut env = ProfileEnv::new(profile, snapshot);
    ///
    /// // The environment must be reset before it can be stepped!
    /// env.reset()?;
    /// ```
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

    /// Rewinds time to the beginning.
    ///
    /// Restores the exact memory snapshot provided during creation, clears the TAS recorder,
    /// flushes the frame stack, and computes the very first observation for the agent.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::SmbControlEnv, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let mut env = SmbControlEnv::from_config(config)?;
    /// // Start a new episode.
    /// let initial_features = env.reset()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if the emulator core rejects the state payload.
    pub fn reset(&mut self) -> Result<P::Features, AiError> {
        self.core.load_state(self.snapshot.snapshot.clone());
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

    /// Commits an action to the universe and fast-forwards time.
    ///
    /// The environment holds the requested joypad buttons down and advances the emulator
    /// by `frame_skip` frames. It then calculates how much progress was made and applies
    /// the psychological reward (or penalty).
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::SmbControlEnv, actions::ControlAction, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let mut env = SmbControlEnv::from_config(config)?;
    /// env.reset()?;
    ///
    /// // Move right and jump!
    /// let output = env.step(ControlAction::RightA)?;
    /// if output.done {
    ///     println!("The agent met its end.");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if called before [`Self::reset`] or if the episode has already ended.
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
    pub fn core_mut(&mut self) -> &mut NesCore {
        &mut self.core
    }

    /// Gazes into the environment's current state.
    ///
    /// Flattens the recent history of visuals and internal game memory into a format
    /// easily digested by the agent's neural network.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::SmbControlEnv, actions::ControlAction, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let mut env = SmbControlEnv::from_config(config)?;
    /// env.reset()?;
    ///
    /// let vision = env.observation()?;
    /// assert_eq!(vision.frames.len(), 4 * 84 * 84); // 4 frames stacked
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`AiError::Unsupported`] if called before the environment is initialized via `reset()`.
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

    /// Returns the complete history of inputs from the current episode.
    ///
    /// Every twitch of the controller is recorded. When the agent does something brilliant,
    /// this TAS movie is the proof we save to disk.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::SmbControlEnv, actions::ControlAction, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let mut env = SmbControlEnv::from_config(config)?;
    /// env.reset()?;
    /// env.step(ControlAction::RightA)?;
    ///
    /// let movie = env.recorded_movie();
    /// println!("Recorded {} frames", movie.total_frames());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn recorded_movie(&self) -> &nes_core::tas::TasMovie {
        self.recorder.movie()
    }

    /// Signs the death certificate.
    ///
    /// Compiles all terminal metrics (frames survived, total reward, final memory hash)
    /// into a tidy piece of metadata for telemetry and debugging.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::SmbControlEnv, actions::ControlAction, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let mut env = SmbControlEnv::from_config(config)?;
    /// env.reset()?;
    /// let meta = env.finish_episode(42.5);
    /// println!("Agent scored: {}", meta.total_reward);
    /// # Ok(())
    /// # }
    /// ```
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

/// A concrete instance of the AI control environment for Super Mario Bros.
pub type SmbControlEnv = ProfileEnv<SmbProfile>;

/// Type-erased wrapper for concrete environment specializations like SMB.
///
/// In Rust, generic types `ProfileEnv<P>` infect your entire API surface. This enum
/// exists to hide that complexity behind a single enum, making it trivial to pass
/// "some control environment" across thread bounds without dealing with `Box<dyn Any>`
/// trait object headaches.
pub enum AnyControlEnv {
    /// Environment variant specifically handling Super Mario Bros. logic.
    Smb(Box<SmbControlEnv>),
}

impl AnyControlEnv {
    /// Summons the appropriate universe based on the provided configuration.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::AnyControlEnv, actions::ControlAction, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// let mut env = AnyControlEnv::from_config(config)?;
    /// env.reset()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if config validation, snapshot loading, or ROM validation fails.
    pub fn from_config(cfg: AiProfileConfig) -> Result<Self, AiError> {
        match cfg.game {
            GameProfileId::Smb => Ok(Self::Smb(Box::new(SmbControlEnv::from_config(cfg)?))),
        }
    }

    /// Rewinds time to the beginning for the active environment.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::AnyControlEnv, actions::ControlAction, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let mut env = AnyControlEnv::from_config(config)?;
    /// env.reset()?; // Ready to play!
    /// # Ok(())
    /// # }
    /// ```
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

    /// Commits an action to the active universe and fast-forwards time.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::AnyControlEnv, actions::ControlAction, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let mut env = AnyControlEnv::from_config(config)?;
    /// # env.reset()?;
    /// let step = env.step(ControlAction::RightB)?;
    /// println!("Score changed by: {}", step.reward.total);
    /// # Ok(())
    /// # }
    /// ```
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

    /// Gazes into the active environment's current state.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::AnyControlEnv, actions::ControlAction, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let mut env = AnyControlEnv::from_config(config)?;
    /// # env.reset()?;
    /// let vision = env.observation()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`AiError`] if called before reset.
    pub fn observation(&self) -> Result<ObservationSnapshot, AiError> {
        match self {
            Self::Smb(env) => env.observation(),
        }
    }

    /// Extracts the recorded TAS input sequence from the active universe.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::AnyControlEnv, actions::ControlAction, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let mut env = AnyControlEnv::from_config(config)?;
    /// # env.reset()?;
    /// let movie = env.recorded_movie();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn recorded_movie(&self) -> &TasMovie {
        match self {
            Self::Smb(env) => env.recorded_movie(),
        }
    }

    /// Signs the death certificate for the active universe, returning metrics.
    ///
    /// ## Examples
    ///
    /// ```rust,no_run
    /// # use nes_ai::{env::AnyControlEnv, actions::ControlAction, config::{AiProfileConfig, ObservationConfig, RewardConfig, GameProfileId}, error::AiError};
    /// # fn main() -> Result<(), AiError> {
    /// # let config = AiProfileConfig { game: GameProfileId::Smb, id: "test".to_string(), rom_path: std::path::PathBuf::new(), snapshot_path: std::path::PathBuf::new(), bootstrap_tas_path: std::path::PathBuf::new(), frame_stack: 4, frame_skip: 4, max_episode_frames: 100, observation: ObservationConfig { width: 84, height: 84 }, reward: RewardConfig { forward_progress: 1.0, alive_bonus: 0.0, stall_penalty: 0.0, death_penalty: 0.0, stall_frames: 10 } };
    /// # let mut env = AnyControlEnv::from_config(config)?;
    /// # env.reset()?;
    /// let meta = env.finish_episode(0.0);
    /// # Ok(())
    /// # }
    /// ```
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
