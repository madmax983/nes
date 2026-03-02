use std::collections::BTreeMap;

use nes_core::{Command, CoreError, CoreSnapshot, NesCore};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollbackConfig {
    pub local_player: u8,
    pub input_delay_frames: u32,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledInput {
    pub frame: u64,
    pub bits: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteInputIngest {
    pub frame: u64,
    pub bits: u8,
    pub rollback_queued: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RollbackStep {
    pub frame: u64,
    pub rollback_distance: u64,
    pub local_bits: u8,
    pub remote_bits: u8,
    pub state_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashComparison {
    PendingLocalFrame,
    Match,
    Mismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackError {
    InvalidLocalPlayer(u8),
    InvalidRollbackConfig {
        input_delay_frames: u32,
        max_rollback_frames: u32,
    },
    MissingSnapshot(u64),
    RollbackWindowExceeded {
        rollback_from: u64,
        next_frame: u64,
        max_rollback_frames: u32,
    },
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

    #[must_use]
    pub fn next_frame(&self) -> u64 {
        self.next_frame
    }

    #[must_use]
    pub fn local_player(&self) -> u8 {
        self.config.local_player
    }

    #[must_use]
    pub fn input_delay_frames(&self) -> u32 {
        self.config.input_delay_frames
    }

    #[must_use]
    pub fn max_rollback_frames(&self) -> u32 {
        self.config.max_rollback_frames
    }

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

    pub fn schedule_local_input(&mut self, bits: u8) -> ScheduledInput {
        let target_frame = self.next_frame + u64::from(self.config.input_delay_frames);
        self.local_inputs.insert(target_frame, bits);
        ScheduledInput {
            frame: target_frame,
            bits,
        }
    }

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

    #[must_use]
    pub fn compare_remote_hash(&self, frame: u64, remote_hash: u64) -> HashComparison {
        match self.frame_hashes.get(&frame).copied() {
            Some(local_hash) if local_hash == remote_hash => HashComparison::Match,
            Some(_) => HashComparison::Mismatch,
            None => HashComparison::PendingLocalFrame,
        }
    }

    #[must_use]
    pub fn frame_hash(&self, frame: u64) -> Option<u64> {
        self.frame_hashes.get(&frame).copied()
    }

    #[must_use]
    pub fn resolved_inputs(&self, frame: u64) -> Option<(u8, u8)> {
        let local = self.resolved_local.get(&frame).copied()?;
        let remote = self.resolved_remote.get(&frame).copied()?;
        Some((local, remote))
    }

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
        let local_bits = self.resolve_bits(frame, true);
        let remote_bits = self.resolve_bits(frame, false);
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

    fn resolve_bits(&self, frame: u64, local: bool) -> u8 {
        let prior = frame
            .checked_sub(1)
            .and_then(|prev| {
                if local {
                    self.resolved_local.get(&prev).copied()
                } else {
                    self.resolved_remote.get(&prev).copied()
                }
            })
            .unwrap_or_default();

        if local {
            self.local_inputs.get(&frame).copied().unwrap_or(prior)
        } else {
            self.remote_inputs.get(&frame).copied().unwrap_or(prior)
        }
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
    use super::{HashComparison, RollbackConfig, RollbackEngine};
    use nes_core::{Command, NesCore};

    pub fn make_core_for_rollback() -> NesCore {
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
}









#[cfg(test)]
mod proptests {
    use super::*;
    use nes_core::NesCore;
    use proptest::prelude::*;
    use crate::rollback::tests::make_core_for_rollback;

    proptest! {
        #[test]
        fn fuzz_rollback_engine(
            local_inputs in proptest::collection::vec(0..255u8, 10..50),
            remote_inputs in proptest::collection::vec(0..255u8, 10..50),
            delay_frames in 0..10u32,
            max_rollback in 1..20u32,
            random_seed in 0..100u32,
        ) {
            let mut core = make_core_for_rollback();
            let config = RollbackConfig {
                local_player: 1,
                input_delay_frames: delay_frames.min(max_rollback),
                max_rollback_frames: max_rollback,
            };
            let mut engine = RollbackEngine::new(config).unwrap();

            let mut frame = 0;
            for (local, remote) in local_inputs.iter().zip(remote_inputs.iter()) {
                engine.schedule_local_input(*local);

                // Add noise to the inputs to fuzz remote inputs behavior
                if random_seed % 3 == 0 {
                    engine.ingest_remote_input(frame, *remote);
                } else if random_seed % 3 == 1 {
                    engine.ingest_remote_input(frame.saturating_add(1), *remote);
                } else {
                    engine.ingest_remote_input(frame.saturating_sub(1), *remote);
                }

                // Call advance frame and ignore potential errors because we intentionally throw bad inputs
                let _ = engine.advance_frame(&mut core);
                frame += 1;
            }
        }
    }
}
