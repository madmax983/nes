//! The impartial referee for competitive NES speedrunning (Real-Time Attack).
//!
//! Speedrunners demand absolute precision and zero-trust verification. This module transforms
//! a simple emulator into a tournament-grade competitive platform by automating the entire
//! speedrun lifecycle. It acts as the "Black Box" flight recorder: it knows exactly when
//! a run starts, it splits with frame-perfect accuracy based on memory inspection, and it
//! violently invalidates the attempt if the player commits a cardinal sin (like rewinding).
//!
//! # The Lore
//!
//! When a user selects a game, the system loads an [`crate::rta::RtaProfile`]. This profile is a contract
//! that dictates exactly which memory addresses define the start, end, and intermediate
//! milestones (splits) of a run.
//!
//! The beating heart of this system is the [`crate::rta::RtaManager`], which must be `.tick()`'d every
//! single frame. It constantly interrogates the emulator's memory to see if any of the profile's
//! [`crate::rta::TriggerRule`] conditions have been met. When the dust settles, the manager spits out
//! a cryptographically sound `.run.json` artifact containing the split times, the runner's
//! controller inputs, and a verdict on the run's legitimacy.
//!
//! # Architecture
//!
//! The RTA engine is built upon these pillars:
//!
//! * **Profiles ([`crate::rta::RtaProfile`]):** The immutable laws of the run, loaded from TOML files.
//! * **Triggers ([`crate::rta::TriggerRule`]):** The exact memory mutations (e.g., "Address `0x071A` becomes `1`") that cause the stopwatch to react.
//! * **The Manager ([`crate::rta::RtaManager`]):** The active state machine that relentlessly enforces the profile rules frame-by-frame.
//! * **Calibration ([`crate::rta::CalibrationRecorder`]):** The automated drafting tool that hallucinates new profiles by statistically analyzing a user's manual splits.

use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const DEFAULT_RTA_PROFILES_DIR: &str = "config/rta/profiles";
pub const DEFAULT_RTA_RUNS_DIR: &str = "runs/rta";

#[derive(Debug, Clone)]
pub struct RtaRuntimeConfig {
    pub profile_id_override: Option<String>,
    pub profiles_dir: PathBuf,
    pub runs_dir: PathBuf,
    pub calibrate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProfileStatus {
    Draft,
    #[default]
    Published,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TimerClock {
    #[default]
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum FocusLossPolicy {
    AutoPause,
    Invalidate,
    #[default]
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ForbiddenAction {
    #[default]
    Rewind,
    SaveLoad,
    FrameStep,
}

impl ForbiddenAction {
    fn as_reason(self) -> &'static str {
        match self {
            Self::Rewind => "rewind",
            Self::SaveLoad => "save_load",
            Self::FrameStep => "frame_step",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TriggerWidth {
    #[default]
    U8,
    U16,
    U32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TriggerOp {
    #[default]
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
    BitSet,
    BitClear,
    Changed,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct TimerPolicy {
    pub clock: TimerClock,
    pub focus_loss: FocusLossPolicy,
    pub manual_fallback: bool,
}

impl Default for TimerPolicy {
    fn default() -> Self {
        Self {
            clock: TimerClock::Wall,
            focus_loss: FocusLossPolicy::Continue,
            manual_fallback: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct InvalidationPolicy {
    pub invalidate_on: Vec<ForbiddenAction>,
}

impl Default for InvalidationPolicy {
    fn default() -> Self {
        Self {
            invalidate_on: vec![
                ForbiddenAction::Rewind,
                ForbiddenAction::SaveLoad,
                ForbiddenAction::FrameStep,
            ],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct SplitPolicy {
    pub append_only: bool,
    pub manual_hotkey: String,
}

impl Default for SplitPolicy {
    fn default() -> Self {
        Self {
            append_only: true,
            manual_hotkey: "F9".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, Default)]
#[serde(default, deny_unknown_fields)]
pub struct LoggingPolicy {
    pub save_input_log: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TriggerRule {
    pub address: u16,
    pub width: TriggerWidth,
    pub op: TriggerOp,
    pub value: u32,
    pub debounce_frames: u32,
    pub require_consecutive: u32,
}

impl Default for TriggerRule {
    fn default() -> Self {
        Self {
            address: 0,
            width: TriggerWidth::U8,
            op: TriggerOp::Eq,
            value: 0,
            debounce_frames: 0,
            require_consecutive: 1,
        }
    }
}

/// An individual rule that defines when a "split" should occur in a speedrun.
///
/// A split typically represents the completion of a level or segment. When the conditions
/// defined in the underlying `TriggerRule` are met, the RTA engine logs the elapsed time.
///
/// # Examples
///
/// ```toml
/// [[splits]]
/// name = "World 1-1"
/// [splits.trigger]
/// address = "0x071A" # Current Screen
/// old_value = 0
/// new_value = 1
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SplitRule {
    /// The human-readable name of the split (e.g., `"World 1-1"`).
    pub name: String,
    /// The conditions required to trigger the split.
    pub trigger: TriggerRule,
}

impl Default for SplitRule {
    fn default() -> Self {
        Self {
            name: "split".to_owned(),
            trigger: TriggerRule::default(),
        }
    }
}

/// Defines the rules, metadata, and triggers for an RTA speedrun.
///
/// Profiles are typically loaded from TOML files (e.g., `smb-any.toml`) within the
/// `config/rta/profiles` directory. They declare what actions are forbidden, how the timer
/// operates, and the memory state transitions required to automatically split a run.
///
/// # Examples
///
/// An example of an `RtaProfile` loaded from TOML:
///
/// ```toml
/// id = "smb-any"
/// game = "Super Mario Bros"
/// category = "Any%"
/// rom_hashes = ["ea343f4e44562066f8114f6e80b2d35c43d3120e71ce001b33edccfa98319df6"]
///
/// [start]
/// address = "0x071A" # Current Screen
/// old_value = 0
/// new_value = 1
