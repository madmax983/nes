//! Deterministic rollback engine for NES netplay.
//!
//! This module provides a GGPO-style rollback netcode implementation for the NES emulator.
//! Rollback netplay allows two remote players to play synchronously with near-zero perceived
//! latency by predictively simulating the remote player's inputs and "rolling back" the emulator
//! state when a misprediction occurs.
//!
//! # The Black Box
//!
//! At its core, the [`RollbackEngine`] is a specialized state manager that wraps around the
//! [`nes_core::NesCore`]. It maintains a history of controller inputs and emulator snapshots.
//! By decoupling the *presentation* frame rate from the *simulation* frame rate, it allows
//! the local simulation to advance seamlessly while waiting on the network, quickly rewriting
//! history when new network packets arrive.
//!
//! # How it works
//!
//! 1. **Input Delay:** A small, fixed input delay is added to local inputs to absorb typical network latency.
//! 2. **Prediction:** If the remote player's input hasn't arrived by the time a frame needs to be rendered,
//!    the engine predicts that they will continue holding the same buttons as the previous frame.
//! 3. **Rollback:** When the actual remote input arrives, if it differs from the prediction, the engine
//!    restores an older snapshot of the emulator state (prior to the misprediction), applies the correct
//!    inputs, and rapidly re-simulates the intervening frames to catch back up to the present.

use std::collections::BTreeMap;

use nes_core::{Command, CoreError, CoreSnapshot, NesCore};

/// Configuration parameters for the [`RollbackEngine`].
///
/// These parameters define the boundaries of the rollback window and how local inputs
/// are delayed to mitigate network jitter.
///
/// ## Examples
///
/// ```
/// use nes_netplay::RollbackConfig;
///
/// let _config = RollbackConfig {
///     local_player: 1, // Player 1
///     input_delay_frames: 2, // Delay local inputs by 2 frames
///     max_rollback_frames: 60, // Keep 1 second of history (at 60 FPS)
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollbackConfig {
    /// Which player controller (1 or 2) the local user is controlling.
    pub local_player: u8,
    /// The number of frames to artificially delay local inputs.
    ///
    /// Setting this higher increases local latency but reduces the frequency and
    /// severity of rollbacks.
    pub input_delay_frames: u32,
    /// The maximum number of frames the engine is allowed to roll back.
    ///
    /// If a remote input arrives later than this window, the session will desync or abort.
    pub max_rollback_frames: u32,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            local_player: 1,
            input_delay_frames: 2,
            max_rollback_frames: 240,
        }
    }
}

/// Represents a local input that has been queued for execution in the future.
///
/// Because local inputs are artificially delayed to mask network latency,
/// they are scheduled for `current_frame + input_delay_frames`. This struct
/// tracks what that input is and when it will actually affect the emulator state.
///
/// ## Examples
///
/// ```
/// use nes_netplay::{RollbackConfig, RollbackEngine};
///
/// let mut engine = RollbackEngine::new(RollbackConfig {
///     local_player: 1,
///     input_delay_frames: 2,
///     max_rollback_frames: 60,
/// }).unwrap();
///
/// // Schedule an input for the present frame + delay
/// let scheduled = engine.schedule_local_input(0x80); // A button
/// assert_eq!(scheduled.frame, 2);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledInput {
    /// The absolute frame number when this input will be executed.
    pub frame: u64,
    /// The controller button bitmask to apply.
    pub bits: u8,
}

/// The result of processing an incoming input from the remote player.
///
/// When remote inputs arrive, they may arrive "late" relative to the local simulation.
/// If they differ from what the engine predicted, a rollback is queued. This struct
/// provides observability into that ingestion process.
///
/// ## Examples
///
/// ```
/// use nes_netplay::{RollbackConfig, RollbackEngine};
/// use nes_core::NesCore;
///
/// let mut engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
/// let mut core = NesCore::new();
///
/// // Advance a few frames, so the engine has predicted remote inputs
/// engine.advance_frame(&mut core).unwrap();
/// engine.advance_frame(&mut core).unwrap();
///
/// // Receive actual input for frame 0 from the peer
/// let ingest = engine.ingest_remote_input(0, 0x01);
/// assert_eq!(ingest.rollback_queued, true); // Assuming the prediction (0x00) didn't match 0x01
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteInputIngest {
    /// The absolute frame number this remote input targets.
    pub frame: u64,
    /// The remote controller button bitmask.
    pub bits: u8,
    /// Whether this input contradicted a previous prediction, meaning the engine
    /// will rewind state on the next frame advance.
    pub rollback_queued: bool,
}

/// A summary of what occurred during a single call to [`RollbackEngine::advance_frame`].
///
/// This provides crucial observability for the netplay host, allowing it to log rollbacks,
/// verify state hashes, and broadcast the actual inputs used.
///
/// ## Examples
///
/// ```
/// use nes_netplay::{RollbackConfig, RollbackEngine};
/// use nes_core::NesCore;
///
/// let mut engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
/// let mut core = NesCore::new();
///
/// let step = engine.advance_frame(&mut core).unwrap();
/// assert_eq!(step.frame, 0);
/// assert_eq!(step.rollback_distance, 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollbackStep {
    /// The frame that was just completed.
    pub frame: u64,
    /// How many frames the engine had to rewind before simulating this frame.
    /// `0` means no rollback occurred.
    pub rollback_distance: u64,
    /// The resolved local input bitmask used for this frame.
    pub local_bits: u8,
    /// The resolved remote input bitmask used for this frame.
    pub remote_bits: u8,
    /// The deterministic hash of the [`nes_core::NesCore`] state immediately after this frame.
    pub state_hash: u64,
}

/// The result of comparing the local state hash against a remote peer's hash.
///
/// This is used to detect unrecoverable desynchronizations between the two clients.
///
/// ## Examples
///
/// ```
/// use nes_netplay::{RollbackConfig, RollbackEngine, HashComparison};
/// use nes_core::NesCore;
///
/// let mut engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
/// let mut core = NesCore::new();
///
/// // Initially, we have no frame hashes
/// assert_eq!(engine.compare_remote_hash(0, 12345), HashComparison::PendingLocalFrame);
///
/// engine.advance_frame(&mut core).unwrap();
///
/// // Once the local frame is processed, it will likely mismatch an arbitrary remote hash
/// assert_eq!(engine.compare_remote_hash(0, 12345), HashComparison::Mismatch);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashComparison {
    /// The local engine has not yet reached the frame in question, so it cannot be compared yet.
    PendingLocalFrame,
    /// The state hashes match. The simulation remains synchronized.
    Match,
    /// The state hashes differ. A desync has occurred, and the session should likely be aborted.
    Mismatch,
}

/// Errors that can occur during rollback engine configuration or execution.
///
/// ## Examples
///
/// ```
/// use nes_netplay::{RollbackConfig, RollbackEngine};
///
/// // Will fail because max_rollback_frames must be >= input_delay_frames
/// let config = RollbackConfig {
///     local_player: 1,
///     input_delay_frames: 60,
///     max_rollback_frames: 10,
/// };
/// let result = RollbackEngine::new(config);
/// assert!(result.is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackError {
    /// The configured `local_player` was not 1 or 2.
    InvalidLocalPlayer(u8),
    /// The rollback configuration parameters are mathematically invalid
    /// (e.g., input delay is larger than the max rollback window).
    InvalidRollbackConfig {
        /// The configured input delay frames.
        input_delay_frames: u32,
        /// The configured maximum rollback frames.
        max_rollback_frames: u32,
    },
    /// The engine attempted to roll back to a frame, but the snapshot for that frame
    /// had already been pruned from history.
    MissingSnapshot(u64),
    /// A remote input arrived so late that rewinding to correct it would exceed
    /// the configured `max_rollback_frames`.
    RollbackWindowExceeded {
        /// The frame the engine needed to return to.
        rollback_from: u64,
        /// The current frame of the engine.
        next_frame: u64,
        /// The maximum allowed rollback distance.
        max_rollback_frames: u32,
    },
    /// An underlying error occurred within the [`nes_core::NesCore`] during simulation.
    Core(CoreError),
}

impl core::fmt::Display for RollbackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidLocalPlayer(player) => {
                write!(f, "local_player must be 1 or 2, got {player}")
            }
            Self::InvalidRollbackConfig {
                input_delay_frames,
                max_rollback_frames,
            } => write!(
                f,
                "invalid rollback config: input_delay_frames={input_delay_frames}, max_rollback_frames={max_rollback_frames}"
            ),
            Self::MissingSnapshot(frame) => {
                write!(f, "missing rollback snapshot for frame {frame}")
            }
            Self::RollbackWindowExceeded {
                rollback_from,
                next_frame,
                max_rollback_frames,
            } => write!(
                f,
                "rollback window exceeded: rollback_from={rollback_from}, next_frame={next_frame}, max={max_rollback_frames}"
            ),
            Self::Core(err) => write!(f, "core error during rollback step: {err}"),
        }
    }
}

impl std::error::Error for RollbackError {}

impl From<CoreError> for RollbackError {
    fn from(value: CoreError) -> Self {
        Self::Core(value)
    }
}

/// A deterministic rollback engine for synchronizing NES execution.
///
/// The `RollbackEngine` coordinates a single [`nes_core::NesCore`] instance, applying delayed local
/// inputs and predictive remote inputs. When mispredictions are identified, it forces the
/// NES core to rewind to a previously saved snapshot and recalculate state.
///
/// ## Examples
///
/// ```
/// use nes_netplay::{RollbackConfig, RollbackEngine};
///
/// let config = RollbackConfig {
///     local_player: 1,
///     input_delay_frames: 2,
///     max_rollback_frames: 60,
/// };
/// let engine = RollbackEngine::new(config).unwrap();
/// ```
#[derive(Debug, Clone)]
pub struct RollbackEngine {
    config: RollbackConfig,
    next_frame: u64,
    pending_rollback_from: Option<u64>,
    local_inputs: BTreeMap<u64, u8>,
    remote_inputs: BTreeMap<u64, u8>,
    resolved_local: BTreeMap<u64, u8>,
    resolved_remote: BTreeMap<u64, u8>,
    snapshots: BTreeMap<u64, CoreSnapshot>,
    frame_hashes: BTreeMap<u64, u64>,
}

impl RollbackEngine {
    /// Creates a new `RollbackEngine` initialized at frame 0.
    ///
    /// ## Panics
    ///
    /// This function does not panic, but it will return a [`RollbackError`] if the
    /// `local_player` is not `1` or `2`, or if the rollback window settings are invalid
    /// (e.g. `max_rollback_frames` is 0 or smaller than `input_delay_frames`).
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine};
    ///
    /// let engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
    /// ```
    pub fn new(config: RollbackConfig) -> Result<Self, RollbackError> {
        if !matches!(config.local_player, 1 | 2) {
            return Err(RollbackError::InvalidLocalPlayer(config.local_player));
        }
        if config.max_rollback_frames == 0 || config.input_delay_frames > config.max_rollback_frames
        {
            return Err(RollbackError::InvalidRollbackConfig {
                input_delay_frames: config.input_delay_frames,
                max_rollback_frames: config.max_rollback_frames,
            });
        }
        Ok(Self {
            config,
            next_frame: 0,
            pending_rollback_from: None,
            local_inputs: BTreeMap::new(),
            remote_inputs: BTreeMap::new(),
            resolved_local: BTreeMap::new(),
            resolved_remote: BTreeMap::new(),
            snapshots: BTreeMap::new(),
            frame_hashes: BTreeMap::new(),
        })
    }

    /// Returns the absolute frame number that the engine expects to simulate next.
    ///
    /// This is the "present" frame from the perspective of the local simulation.
    /// Note that local inputs scheduled now will actually apply to `next_frame + input_delay_frames`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine};
    ///
    /// let engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
    /// assert_eq!(engine.next_frame(), 0);
    /// ```
    #[must_use]
    pub fn next_frame(&self) -> u64 {
        self.next_frame
    }

    /// Returns the assigned player number (1 or 2) that this engine instance is controlling.
    ///
    /// The remote player will automatically control the other.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine};
    ///
    /// let engine = RollbackEngine::new(RollbackConfig { local_player: 2, ..Default::default() }).unwrap();
    /// assert_eq!(engine.local_player(), 2);
    /// ```
    #[must_use]
    pub fn local_player(&self) -> u8 {
        self.config.local_player
    }

    /// Returns the currently configured input delay in frames.
    ///
    /// By checking this value, the networking layer can determine how far into the
    /// future newly generated local inputs should be scheduled.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine};
    ///
    /// let engine = RollbackEngine::new(RollbackConfig { input_delay_frames: 4, ..Default::default() }).unwrap();
    /// assert_eq!(engine.input_delay_frames(), 4);
    /// ```
    #[must_use]
    pub fn input_delay_frames(&self) -> u32 {
        self.config.input_delay_frames
    }

    /// Returns the maximum allowable rollback window in frames.
    ///
    /// If an incoming remote input indicates a misprediction occurred further in the past
    /// than this window, the simulation will crash and desync.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine};
    ///
    /// let engine = RollbackEngine::new(RollbackConfig { max_rollback_frames: 120, ..Default::default() }).unwrap();
    /// assert_eq!(engine.max_rollback_frames(), 120);
    /// ```
    #[must_use]
    pub fn max_rollback_frames(&self) -> u32 {
        self.config.max_rollback_frames
    }

    /// Dynamically adjusts the input delay at runtime.
    ///
    /// Network latency is rarely constant. This method allows the netplay layer
    /// to continually tune the input delay during a match. If ping spikes, increasing
    /// the delay will introduce more local latency but drastically reduce jarring rollbacks.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine};
    ///
    /// let mut engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
    /// // Network conditions worsened, increase delay to 5 frames
    /// engine.set_input_delay_frames(5).unwrap();
    /// assert_eq!(engine.input_delay_frames(), 5);
    /// ```
    ///
    /// ## Errors
    /// Returns an [`RollbackError::InvalidRollbackConfig`] if the requested delay
    /// exceeds the configured `max_rollback_frames`.
    pub fn set_input_delay_frames(&mut self, input_delay_frames: u32) -> Result<(), RollbackError> {
        if input_delay_frames > self.config.max_rollback_frames {
            return Err(RollbackError::InvalidRollbackConfig {
                input_delay_frames,
                max_rollback_frames: self.config.max_rollback_frames,
            });
        }
        self.config.input_delay_frames = input_delay_frames;
        Ok(())
    }

    /// Schedules a local input to be applied in the future.
    ///
    /// The input is artificially delayed by the configured `input_delay_frames` to absorb
    /// network latency before being sent to the remote peer. The engine will not execute
    /// this input until `advance_frame` reaches the target frame.
    ///
    /// The `bits` parameter should be a bitmask representing standard NES controller
    /// button states.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine};
    ///
    /// let mut engine = RollbackEngine::new(RollbackConfig {
    ///     local_player: 1,
    ///     input_delay_frames: 2,
    ///     max_rollback_frames: 60,
    /// }).unwrap();
    ///
    /// // Schedule an input for the present frame + delay
    /// let scheduled = engine.schedule_local_input(0x80); // A button
    /// assert_eq!(scheduled.frame, 2);
    /// ```
    pub fn schedule_local_input(&mut self, bits: u8) -> ScheduledInput {
        let target_frame = self.next_frame + u64::from(self.config.input_delay_frames);
        self.local_inputs.insert(target_frame, bits);
        ScheduledInput {
            frame: target_frame,
            bits,
        }
    }

    /// Receives an input sent by the remote player and checks if it causes a misprediction.
    ///
    /// If the `frame` represents a time in the past (before `next_frame`), the engine
    /// checks its prediction against the received `bits`. If there is a mismatch, the engine
    /// queues a rollback to be executed on the next call to [`RollbackEngine::advance_frame`].
    ///
    /// Returns a [`RemoteInputIngest`] describing whether a rollback was triggered.
    ///
    /// The `frame` is the absolute frame number the input applies to, and `bits` is the standard
    /// NES controller bitmask.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine};
    /// use nes_core::NesCore;
    ///
    /// let mut engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
    /// let mut core = NesCore::new();
    ///
    /// // Advance a few frames, so the engine has predicted remote inputs
    /// engine.advance_frame(&mut core).unwrap();
    /// engine.advance_frame(&mut core).unwrap();
    ///
    /// // Receive actual input for frame 0 from the peer
    /// let ingest = engine.ingest_remote_input(0, 0x01);
    /// assert_eq!(ingest.rollback_queued, true); // Assuming the prediction (0x00) didn't match 0x01
    /// ```
    pub fn ingest_remote_input(&mut self, frame: u64, bits: u8) -> RemoteInputIngest {
        self.remote_inputs.insert(frame, bits);
        let rollback_queued = if frame < self.next_frame {
            let predicted = self
                .resolved_remote
                .get(&frame)
                .copied()
                .unwrap_or_default();
            if predicted != bits {
                self.queue_rollback(frame);
                true
            } else {
                false
            }
        } else {
            false
        };
        RemoteInputIngest {
            frame,
            bits,
            rollback_queued,
        }
    }

    /// Compares a remote player's state hash for a specific frame against the local state hash.
    ///
    /// Used to detect desyncs between clients.
    ///
    /// Returns a [`HashComparison`] enum indicating a Match, a Mismatch, or if the
    /// local engine hasn't reached `frame` yet.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine, HashComparison};
    /// use nes_core::NesCore;
    ///
    /// let mut engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
    /// let mut core = NesCore::new();
    ///
    /// // Initially, we have no frame hashes
    /// assert_eq!(engine.compare_remote_hash(0, 12345), HashComparison::PendingLocalFrame);
    ///
    /// engine.advance_frame(&mut core).unwrap();
    ///
    /// // Once the local frame is processed, it will likely mismatch an arbitrary remote hash
    /// assert_eq!(engine.compare_remote_hash(0, 12345), HashComparison::Mismatch);
    /// ```
    #[must_use]
    pub fn compare_remote_hash(&self, frame: u64, remote_hash: u64) -> HashComparison {
        match self.frame_hashes.get(&frame).copied() {
            Some(local_hash) if local_hash == remote_hash => HashComparison::Match,
            Some(_) => HashComparison::Mismatch,
            None => HashComparison::PendingLocalFrame,
        }
    }

    /// Retrieves the deterministic hash of the local state for a specific past frame.
    ///
    /// The netplay host should send this value to the peer so they can compare
    /// it against their own using [`compare_remote_hash`](Self::compare_remote_hash).
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine};
    /// use nes_core::NesCore;
    ///
    /// let mut engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
    /// let mut core = NesCore::new();
    ///
    /// // Simulate a frame
    /// engine.advance_frame(&mut core).unwrap();
    ///
    /// // The hash for frame 0 is now available
    /// assert!(engine.frame_hash(0).is_some());
    /// ```
    ///
    /// ## Returns
    ///
    /// Returns `Some(hash)` if the local engine has already simulated that frame.
    /// If it hasn't simulated the frame yet, or if it was pruned from history,
    /// returns `None`.
    #[must_use]
    pub fn frame_hash(&self, frame: u64) -> Option<u64> {
        self.frame_hashes.get(&frame).copied()
    }

    /// Gets the resolved inputs (local and remote) applied during a specific past frame.
    ///
    /// This returns an `Option` containing a tuple of `(local_bits, remote_bits)`.
    /// If the frame has not yet been simulated, it returns `None`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine};
    /// use nes_core::NesCore;
    ///
    /// let mut engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
    /// let mut core = NesCore::new();
    ///
    /// // Initially, no inputs are resolved
    /// assert_eq!(engine.resolved_inputs(0), None);
    ///
    /// engine.advance_frame(&mut core).unwrap();
    ///
    /// // After advancing, the inputs applied on frame 0 are known
    /// assert_eq!(engine.resolved_inputs(0), Some((0, 0))); // 0 for no buttons pressed
    /// ```
    #[must_use]
    pub fn resolved_inputs(&self, frame: u64) -> Option<(u8, u8)> {
        let local = self.resolved_local.get(&frame).copied()?;
        let remote = self.resolved_remote.get(&frame).copied()?;
        Some((local, remote))
    }

    /// Steps the engine forward by one frame, automatically handling any queued rollbacks.
    ///
    /// This function performs the core rollback simulation. If a misprediction was detected via
    /// [`RollbackEngine::ingest_remote_input`], this function will transparently restore a previous state
    /// of the [`NesCore`], rapidly fast-forward simulation through the mispredicted frames,
    /// and then execute the actual frame representing the present time.
    ///
    /// If no rollback is pending, it simply simulates one frame normally.
    ///
    /// It returns a [`RollbackStep`] describing the inputs applied, the hash of the resulting state,
    /// and whether a rollback occurred (and how far it reached back).
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_netplay::{RollbackConfig, RollbackEngine};
    /// use nes_core::NesCore;
    ///
    /// let mut engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
    /// let mut core = NesCore::new();
    ///
    /// let step = engine.advance_frame(&mut core).unwrap();
    /// assert_eq!(step.frame, 0);
    /// assert_eq!(step.rollback_distance, 0);
    /// ```
    pub fn advance_frame(&mut self, core: &mut NesCore) -> Result<RollbackStep, RollbackError> {
        let rollback_distance = if let Some(rollback_from) = self.pending_rollback_from.take() {
            self.rollback_from(core, rollback_from)?;
            self.next_frame.saturating_sub(rollback_from)
        } else {
            0
        };

        let frame = self.next_frame;
        let (local_bits, remote_bits, state_hash) = self.simulate_frame(core, frame)?;
        self.next_frame = self.next_frame.saturating_add(1);
        self.prune_history();

        Ok(RollbackStep {
            frame,
            rollback_distance,
            local_bits,
            remote_bits,
            state_hash,
        })
    }

    fn queue_rollback(&mut self, frame: u64) {
        self.pending_rollback_from = Some(
            self.pending_rollback_from
                .map_or(frame, |current| current.min(frame)),
        );
    }

    fn rollback_from(&mut self, core: &mut NesCore, start_frame: u64) -> Result<(), RollbackError> {
        let rollback_span = self.next_frame.saturating_sub(start_frame);
        if rollback_span > u64::from(self.config.max_rollback_frames) {
            return Err(RollbackError::RollbackWindowExceeded {
                rollback_from: start_frame,
                next_frame: self.next_frame,
                max_rollback_frames: self.config.max_rollback_frames,
            });
        }

        let snapshot = self
            .snapshots
            .get(&start_frame)
            .cloned()
            .ok_or(RollbackError::MissingSnapshot(start_frame))?;
        core.load_state(&snapshot);
        self.clear_from(start_frame);

        for frame in start_frame..self.next_frame {
            let _ = self.simulate_frame(core, frame)?;
        }
        Ok(())
    }

    fn simulate_frame(
        &mut self,
        core: &mut NesCore,
        frame: u64,
    ) -> Result<(u8, u8, u64), RollbackError> {
        self.snapshots.insert(frame, core.save_state());
        let local_bits = self.resolve_local_bits(frame);
        let remote_bits = self.resolve_remote_bits(frame);
        self.resolved_local.insert(frame, local_bits);
        self.resolved_remote.insert(frame, remote_bits);

        if self.config.local_player == 1 {
            core.execute(Command::SetControllerState(local_bits))?;
            core.execute(Command::SetController2State(remote_bits))?;
        } else {
            core.execute(Command::SetControllerState(remote_bits))?;
            core.execute(Command::SetController2State(local_bits))?;
        }
        core.execute(Command::StepFrame)?;

        let hash = core.state_hash();
        self.frame_hashes.insert(frame, hash);
        Ok((local_bits, remote_bits, hash))
    }

    fn resolve_local_bits(&self, frame: u64) -> u8 {
        let prior = frame
            .checked_sub(1)
            .and_then(|prev| self.resolved_local.get(&prev).copied())
            .unwrap_or_default();
        self.local_inputs.get(&frame).copied().unwrap_or(prior)
    }

    fn resolve_remote_bits(&self, frame: u64) -> u8 {
        let prior = frame
            .checked_sub(1)
            .and_then(|prev| self.resolved_remote.get(&prev).copied())
            .unwrap_or_default();
        self.remote_inputs.get(&frame).copied().unwrap_or(prior)
    }

    fn clear_from(&mut self, frame: u64) {
        self.snapshots.split_off(&frame);
        self.resolved_local.split_off(&frame);
        self.resolved_remote.split_off(&frame);
        self.frame_hashes.split_off(&frame);
    }

    fn prune_history(&mut self) {
        let keep_from = self
            .next_frame
            .saturating_sub(u64::from(self.config.max_rollback_frames));
        prune_before(&mut self.snapshots, keep_from);
        prune_before(&mut self.local_inputs, keep_from);
        prune_before(&mut self.remote_inputs, keep_from);
        prune_before(&mut self.resolved_local, keep_from);
        prune_before(&mut self.resolved_remote, keep_from);
        prune_before(&mut self.frame_hashes, keep_from);
    }
}

fn prune_before<T>(map: &mut BTreeMap<u64, T>, keep_from: u64) {
    while let Some((frame, _)) = map.first_key_value() {
        if *frame >= keep_from {
            break;
        }
        map.pop_first();
    }
}

#[cfg(test)]
mod tests {
    use super::{HashComparison, RollbackConfig, RollbackEngine, RollbackError};
    use nes_core::{Command, NesCore};

    fn make_core_for_rollback() -> NesCore {
        let mut core = NesCore::new();
        core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]); // NOP ; JMP $C000
        core
    }

    #[test]
    fn rollback_engine_predicts_and_repairs_remote_input() {
        let mut core = make_core_for_rollback();
        let mut engine = RollbackEngine::new(RollbackConfig {
            local_player: 1,
            input_delay_frames: 0,
            max_rollback_frames: 120,
        })
        .expect("valid config");

        for _ in 0..6 {
            let _ = engine.schedule_local_input(0);
            let step = engine.advance_frame(&mut core).expect("step");
            assert_eq!(step.rollback_distance, 0);
        }

        let late_remote_bits = 0x01;
        let ingest = engine.ingest_remote_input(2, late_remote_bits);
        assert!(ingest.rollback_queued);

        let _ = engine.schedule_local_input(0);
        let repaired = engine.advance_frame(&mut core).expect("rollback step");
        assert!(repaired.rollback_distance >= 4);
        let (_, resolved_remote) = engine.resolved_inputs(2).expect("resolved frame 2");
        assert_eq!(resolved_remote, late_remote_bits);
    }

    #[test]
    fn rollback_engine_hash_comparison_reports_match_and_mismatch() {
        let mut core = make_core_for_rollback();
        let mut engine = RollbackEngine::new(RollbackConfig::default()).expect("valid config");
        let _ = engine.schedule_local_input(0);
        let step = engine.advance_frame(&mut core).expect("step");

        assert_eq!(
            engine.compare_remote_hash(step.frame, step.state_hash),
            HashComparison::Match
        );
        assert_eq!(
            engine.compare_remote_hash(step.frame, step.state_hash ^ 0xDEAD_BEEF),
            HashComparison::Mismatch
        );
        assert_eq!(
            engine.compare_remote_hash(step.frame + 120, 0),
            HashComparison::PendingLocalFrame
        );
    }

    #[test]
    fn rollback_engine_applies_local_player_two_mapping() {
        let mut core = make_core_for_rollback();
        let mut engine = RollbackEngine::new(RollbackConfig {
            local_player: 2,
            input_delay_frames: 0,
            max_rollback_frames: 60,
        })
        .expect("valid config");

        let _ = engine.schedule_local_input(0x80);
        let _ = engine.ingest_remote_input(0, 0x01);
        let step = engine.advance_frame(&mut core).expect("step");
        assert_eq!(step.local_bits, 0x80);
        assert_eq!(step.remote_bits, 0x01);
        assert_eq!(core.controller_bits(), 0x01);
        assert_eq!(core.controller2_bits(), 0x80);

        core.execute(Command::StepFrame).expect("core still valid");
    }

    #[test]
    fn rollback_engine_can_update_input_delay_within_window() {
        let mut engine = RollbackEngine::new(RollbackConfig {
            local_player: 1,
            input_delay_frames: 2,
            max_rollback_frames: 30,
        })
        .expect("valid config");
        assert_eq!(engine.input_delay_frames(), 2);

        engine
            .set_input_delay_frames(6)
            .expect("delay within rollback window");
        assert_eq!(engine.input_delay_frames(), 6);

        let err = engine
            .set_input_delay_frames(31)
            .expect_err("delay larger than rollback window should fail");
        assert!(matches!(
            err,
            super::RollbackError::InvalidRollbackConfig { .. }
        ));
    }

    #[test]
    fn rollback_engine_exposes_initial_config_and_state() {
        let engine = RollbackEngine::new(RollbackConfig {
            local_player: 2,
            input_delay_frames: 3,
            max_rollback_frames: 42,
        })
        .expect("valid config");
        assert_eq!(engine.next_frame(), 0);
        assert_eq!(engine.local_player(), 2);
        assert_eq!(engine.input_delay_frames(), 3);
        assert_eq!(engine.max_rollback_frames(), 42);
    }

    #[test]
    fn rollback_engine_allows_input_delay_equal_to_rollback_window() {
        let mut engine = RollbackEngine::new(RollbackConfig {
            local_player: 1,
            input_delay_frames: 1,
            max_rollback_frames: 4,
        })
        .expect("valid config");
        engine
            .set_input_delay_frames(4)
            .expect("equal delay and rollback window should be valid");
        assert_eq!(engine.input_delay_frames(), 4);
    }

    #[test]
    fn rollback_engine_schedule_local_input_applies_delay_to_target_frame() {
        let mut engine = RollbackEngine::new(RollbackConfig {
            local_player: 1,
            input_delay_frames: 2,
            max_rollback_frames: 10,
        })
        .expect("valid config");
        let scheduled = engine.schedule_local_input(0x3C);
        assert_eq!(scheduled.frame, 2);
        assert_eq!(scheduled.bits, 0x3C);
    }

    #[test]
    fn rollback_engine_frame_hash_and_resolved_inputs_are_queryable() {
        let mut core = make_core_for_rollback();
        let mut engine = RollbackEngine::new(RollbackConfig {
            local_player: 1,
            input_delay_frames: 0,
            max_rollback_frames: 60,
        })
        .expect("valid config");

        assert_eq!(engine.frame_hash(0), None);
        assert_eq!(engine.resolved_inputs(0), None);

        let _ = engine.schedule_local_input(0x11);
        let _ = engine.ingest_remote_input(0, 0x22);
        let step = engine.advance_frame(&mut core).expect("step");

        assert_eq!(step.frame, 0);
        assert_eq!(engine.frame_hash(0), Some(step.state_hash));
        assert_eq!(engine.resolved_inputs(0), Some((0x11, 0x22)));
    }

    #[test]
    fn rollback_engine_rejects_rollback_beyond_configured_window() {
        let mut core = make_core_for_rollback();
        let mut engine = RollbackEngine::new(RollbackConfig {
            local_player: 1,
            input_delay_frames: 0,
            max_rollback_frames: 2,
        })
        .expect("valid config");

        for _ in 0..5 {
            let _ = engine.schedule_local_input(0);
            let _ = engine.advance_frame(&mut core).expect("advance");
        }
        let _ = engine.ingest_remote_input(0, 0x01);
        let _ = engine.schedule_local_input(0);
        let err = engine
            .advance_frame(&mut core)
            .expect_err("rollback beyond max window should fail");

        match err {
            RollbackError::RollbackWindowExceeded {
                rollback_from,
                next_frame,
                max_rollback_frames,
            } => {
                assert_eq!(rollback_from, 0);
                assert_eq!(next_frame, 5);
                assert_eq!(max_rollback_frames, 2);
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn rollback_engine_allows_rollback_exactly_at_configured_window() {
        let mut core = make_core_for_rollback();
        let mut engine = RollbackEngine::new(RollbackConfig {
            local_player: 1,
            input_delay_frames: 0,
            max_rollback_frames: 2,
        })
        .expect("valid config");

        for _ in 0..2 {
            let _ = engine.schedule_local_input(0);
            let _ = engine.advance_frame(&mut core).expect("advance");
        }

        let ingest = engine.ingest_remote_input(0, 0x01);
        assert!(ingest.rollback_queued);

        let _ = engine.schedule_local_input(0);
        let step = engine
            .advance_frame(&mut core)
            .expect("rollback equal to window should be allowed");

        assert_eq!(step.frame, 2);
        assert_eq!(step.rollback_distance, 2);
    }

    #[test]
    fn rollback_engine_prunes_old_hashes_after_advance() {
        let mut core = make_core_for_rollback();
        let mut engine = RollbackEngine::new(RollbackConfig {
            local_player: 1,
            input_delay_frames: 0,
            max_rollback_frames: 2,
        })
        .expect("valid config");

        for _ in 0..5 {
            let _ = engine.schedule_local_input(0);
            let _ = engine.advance_frame(&mut core).expect("advance");
        }

        assert_eq!(engine.next_frame(), 5);
        assert_eq!(engine.frame_hash(0), None);
        assert_eq!(engine.frame_hash(1), None);
        assert!(engine.frame_hash(4).is_some());
    }

    #[test]
    fn rollback_error_display_messages_include_context() {
        let invalid_player = RollbackError::InvalidLocalPlayer(9).to_string();
        assert!(invalid_player.contains("1 or 2"));
        assert!(invalid_player.contains("9"));

        let invalid_config = RollbackError::InvalidRollbackConfig {
            input_delay_frames: 7,
            max_rollback_frames: 3,
        }
        .to_string();
        assert!(invalid_config.contains("input_delay_frames=7"));
        assert!(invalid_config.contains("max_rollback_frames=3"));

        let missing_snapshot = RollbackError::MissingSnapshot(12).to_string();
        assert!(missing_snapshot.contains("frame 12"));

        let window_exceeded = RollbackError::RollbackWindowExceeded {
            rollback_from: 10,
            next_frame: 20,
            max_rollback_frames: 5,
        }
        .to_string();
        assert!(window_exceeded.contains("rollback_from=10"));
        assert!(window_exceeded.contains("next_frame=20"));
        assert!(window_exceeded.contains("max=5"));

        let core_error = RollbackError::Core(nes_core::CoreError::UnsupportedCommand).to_string();
        assert!(core_error.contains("unsupported command"));
    }

    #[test]
    fn rollback_engine_clears_from_frame_on_rollback() {
        let core = make_core_for_rollback();

        let mut manual_engine = RollbackEngine::new(RollbackConfig::default()).unwrap();
        manual_engine.snapshots.insert(1, core.save_state());
        manual_engine.snapshots.insert(2, core.save_state());
        manual_engine.snapshots.insert(3, core.save_state());
        manual_engine.resolved_local.insert(1, 0);
        manual_engine.resolved_local.insert(2, 0);
        manual_engine.resolved_local.insert(3, 0);
        manual_engine.resolved_remote.insert(1, 0);
        manual_engine.resolved_remote.insert(2, 0);
        manual_engine.resolved_remote.insert(3, 0);
        manual_engine.frame_hashes.insert(1, 0);
        manual_engine.frame_hashes.insert(2, 0);
        manual_engine.frame_hashes.insert(3, 0);

        manual_engine.clear_from(2);

        assert!(manual_engine.snapshots.contains_key(&1));
        assert!(!manual_engine.snapshots.contains_key(&2));
        assert!(!manual_engine.snapshots.contains_key(&3));

        assert!(manual_engine.resolved_local.contains_key(&1));
        assert!(!manual_engine.resolved_local.contains_key(&2));
        assert!(!manual_engine.resolved_local.contains_key(&3));

        assert!(manual_engine.resolved_remote.contains_key(&1));
        assert!(!manual_engine.resolved_remote.contains_key(&2));
        assert!(!manual_engine.resolved_remote.contains_key(&3));

        assert!(manual_engine.frame_hashes.contains_key(&1));
        assert!(!manual_engine.frame_hashes.contains_key(&2));
        assert!(!manual_engine.frame_hashes.contains_key(&3));
    }
}
