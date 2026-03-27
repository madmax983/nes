//! The Real-Time Attack (RTA) engine for speedrunning.
//!
//! This module provides a state machine and configuration system for automating
//! speedrun timing, splitting, and rule enforcement. It relies on evaluating memory
//! values against predefined conditions to track the progress of a run.
//!
//! # Architecture
//!
//! The RTA engine is built around three core concepts:
//!
//! * **Profiles ([`RtaProfile`]):** Static configurations loaded from TOML files that
//!   define when a run starts, ends, pauses, and splits, as well as what actions
//!   (like rewinding) invalidate the run.
//! * **Triggers ([`TriggerRule`]):** Conditions evaluated against the emulator's memory
//!   state (e.g., "Start the timer when memory address `0x071A` becomes `1`").
//! * **The Manager ([`RtaManager`]):** The active state machine that ticks alongside the
//!   emulator frame-by-frame, evaluating triggers and logging events.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
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
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct RtaProfile {
    /// A unique identifier for the profile (e.g., `"smb-any"`).
    pub id: String,
    /// The name of the game (e.g., `"Super Mario Bros"`).
    pub game: Option<String>,
    /// The speedrunning category (e.g., `"Any%"`).
    pub category: Option<String>,
    /// The version of the profile logic.
    pub version: Option<String>,
    /// Whether this is a Draft (under calibration) or a Published run.
    pub status: ProfileStatus,
    /// Authorized SHA-256 ROM hashes that may execute this run.
    pub rom_hashes: Vec<String>,
    /// Rules for when to pause or continue the timer.
    pub timer: TimerPolicy,
    /// Rules that determine what actions invalidate the run.
    pub invalidation: InvalidationPolicy,
    /// Splitting rules.
    pub split_policy: SplitPolicy,
    /// Configuration for artifacts generated post-run.
    pub logging: LoggingPolicy,
    /// The memory trigger that officially starts the timer.
    pub start: TriggerRule,
    /// Optional memory trigger that pauses the timer.
    pub pause: Option<TriggerRule>,
    /// Optional memory trigger that resumes a paused timer.
    pub resume: Option<TriggerRule>,
    /// The memory trigger that marks the end of the run.
    pub end: TriggerRule,
    /// An ordered list of memory triggers that automatically log a split time.
    pub splits: Vec<SplitRule>,
}

impl Default for RtaProfile {
    fn default() -> Self {
        Self {
            id: String::new(),
            game: None,
            category: None,
            version: None,
            status: ProfileStatus::Published,
            rom_hashes: Vec::new(),
            timer: TimerPolicy::default(),
            invalidation: InvalidationPolicy::default(),
            split_policy: SplitPolicy::default(),
            logging: LoggingPolicy::default(),
            start: TriggerRule::default(),
            pause: None,
            resume: None,
            end: TriggerRule {
                address: 1,
                ..TriggerRule::default()
            },
            splits: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LoadedProfile {
    pub path: PathBuf,
    pub profile: RtaProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileSelectionSource {
    AutoByRomHash,
    ManualOverride,
}

#[derive(Debug, Clone)]
pub struct ProfileSelection {
    pub selected: LoadedProfile,
    pub source: ProfileSelectionSource,
}

/// ⚡ Bolt Optimization:
/// Eliminates 32 intermediate `String` heap allocations per ROM hash computation
/// by using `std::fmt::Write` to format directly into the pre-allocated string buffer,
/// rather than creating a temporary formatted `String` for each byte and pushing it.
pub fn compute_rom_hash(rom_bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut hasher = Sha256::new();
    hasher.update(rom_bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// **Performance optimization:** Directly compares strings case-insensitively, avoiding
/// unnecessary heap allocations that would occur from creating lowercased `String` copies.
pub fn compare_rom_hashes(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

/// Loads all `.toml` RTA profiles from the specified directory.
///
/// If a profile file cannot be parsed or the directory does not exist,
/// an error message is returned. The returned profiles are sorted
/// alphabetically by their `id`.
///
/// # Examples
///
/// ```no_run
/// use nes_desktop::rta::load_profiles;
/// use std::path::Path;
///
/// let profiles = load_profiles(Path::new("config/rta/profiles"))
///     .expect("Failed to load profiles");
/// println!("Loaded {} speedrun profiles", profiles.len());
/// ```
pub fn load_profiles(dir: &Path) -> Result<Vec<LoadedProfile>, String> {
    if !dir.exists() {
        return Err(format!(
            "RTA profiles directory '{}' does not exist",
            dir.display()
        ));
    }
    let read_dir = fs::read_dir(dir)
        .map_err(|err| format!("failed to read RTA profiles dir '{}': {err}", dir.display()))?;

    let mut profiles = Vec::<LoadedProfile>::new();
    for entry in read_dir {
        let entry =
            entry.map_err(|err| format!("failed to read RTA profile directory entry: {err}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("toml") {
            continue;
        }
        let bytes = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read RTA profile '{}': {err}", path.display()))?;
        let mut profile = toml::from_str::<RtaProfile>(&bytes)
            .map_err(|err| format!("failed to parse RTA profile '{}': {err}", path.display()))?;
        if profile.id.trim().is_empty() {
            profile.id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("profile")
                .to_owned();
        }
        profiles.push(LoadedProfile { path, profile });
    }
    profiles.sort_by(|lhs, rhs| lhs.profile.id.cmp(&rhs.profile.id));
    Ok(profiles)
}

/// Selects an RTA profile given a ROM hash, or uses a manual override.
///
/// If a `manual_override` profile ID is provided, it takes precedence over
/// auto-selection via ROM hash. If no override is provided, the engine looks
/// for a profile whose authorized `rom_hashes` includes the provided hash.
///
/// If `allow_draft` is false, selecting a profile with a `ProfileStatus::Draft`
/// status will result in an error.
///
/// # Examples
///
/// ```no_run
/// use nes_desktop::rta::{load_profiles, select_profile, ProfileSelectionSource};
/// use std::path::Path;
///
/// let profiles = load_profiles(Path::new("config/rta/profiles")).unwrap();
/// let selection = select_profile(&profiles, "ea343f4e4...", None, false)
///     .expect("Failed to auto-select profile");
///
/// assert_eq!(selection.source, ProfileSelectionSource::AutoByRomHash);
/// println!("Selected profile: {}", selection.selected.profile.id);
/// ```
///
/// ⚡ Bolt Optimization:
/// Instead of filtering the profile list and collecting matches into a `Vec`,
/// this implementation directly uses iterators with `.next()` to search for
/// matches. This completely eliminates an unnecessary heap allocation on the
/// hot path when a single matching profile is found, reducing memory pressure.
pub fn select_profile(
    profiles: &[LoadedProfile],
    rom_hash: &str,
    manual_override: Option<&str>,
    allow_draft: bool,
) -> Result<ProfileSelection, String> {
    if let Some(override_id) = manual_override {
        let Some(found) = profiles
            .iter()
            .find(|profile| profile.profile.id == override_id)
            .cloned()
        else {
            return Err(format!(
                "RTA manual profile override '{}' was not found",
                override_id
            ));
        };
        if !allow_draft && found.profile.status == ProfileStatus::Draft {
            return Err(format!(
                "RTA profile '{}' is draft and cannot be used in strict mode",
                found.profile.id
            ));
        }
        return Ok(ProfileSelection {
            selected: found,
            source: ProfileSelectionSource::ManualOverride,
        });
    }

    let mut match_iter = profiles.iter().filter(|profile| {
        profile
            .profile
            .rom_hashes
            .iter()
            .any(|value| compare_rom_hashes(value, rom_hash))
    });

    let Some(first_match) = match_iter.next() else {
        let known = profiles
            .iter()
            .map(|profile| profile.profile.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "No RTA profile matched ROM hash {rom_hash}. Known profiles: [{}]",
            known
        ));
    };

    if let Some(second_match) = match_iter.next() {
        let mut conflict_names = vec![
            first_match.profile.id.as_str(),
            second_match.profile.id.as_str(),
        ];
        conflict_names.extend(match_iter.map(|profile| profile.profile.id.as_str()));
        let conflict = conflict_names.join(", ");
        return Err(format!(
            "Multiple RTA profiles matched ROM hash {rom_hash}: {conflict}"
        ));
    }

    let selected = first_match.clone();
    if !allow_draft && selected.profile.status == ProfileStatus::Draft {
        return Err(format!(
            "RTA profile '{}' is draft and cannot be used in strict mode",
            selected.profile.id
        ));
    }

    Ok(ProfileSelection {
        selected,
        source: ProfileSelectionSource::AutoByRomHash,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtaSessionState {
    Idle,
    Armed,
    Running,
    Finished,
    InvalidPractice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TriggerSlot {
    Start,
    Pause,
    Resume,
    End,
    Split(usize),
}

#[derive(Debug, Clone)]
struct TriggerRuntime {
    rule: TriggerRule,
    cooldown: u32,
    consecutive_hits: u32,
    last_observed_value: Option<u32>,
}

impl TriggerRuntime {
    fn new(rule: TriggerRule) -> Self {
        Self {
            rule,
            cooldown: 0,
            consecutive_hits: 0,
            last_observed_value: None,
        }
    }

    fn evaluate<F>(&mut self, mut read_u8: F) -> bool
    where
        F: FnMut(u16) -> u8,
    {
        if self.cooldown > 0 {
            self.cooldown = self.cooldown.saturating_sub(1);
        }

        let current = read_memory_value(&self.rule, &mut read_u8);
        let matched = evaluate_trigger_rule(
            self.rule.op,
            current,
            self.rule.value,
            self.last_observed_value,
        );

        self.last_observed_value = Some(current);

        if !matched {
            self.consecutive_hits = 0;
            return false;
        }

        let required = self.rule.require_consecutive.max(1);
        self.consecutive_hits = self.consecutive_hits.saturating_add(1);
        if self.consecutive_hits < required {
            return false;
        }
        if self.cooldown > 0 {
            return false;
        }

        self.consecutive_hits = 0;
        self.cooldown = self.rule.debounce_frames;
        true
    }
}

fn evaluate_trigger_rule(op: TriggerOp, current: u32, value: u32, previous: Option<u32>) -> bool {
    match op {
        TriggerOp::Eq => current == value,
        TriggerOp::Ne => current != value,
        TriggerOp::Gt => current > value,
        TriggerOp::Gte => current >= value,
        TriggerOp::Lt => current < value,
        TriggerOp::Lte => current <= value,
        TriggerOp::BitSet => current & value == value,
        TriggerOp::BitClear => current & value == 0,
        TriggerOp::Changed => previous.is_some_and(|prior| prior != current),
    }
}

fn read_memory_value<F>(rule: &TriggerRule, read_u8: &mut F) -> u32
where
    F: FnMut(u16) -> u8,
{
    match rule.width {
        TriggerWidth::U8 => u32::from(read_u8(rule.address)),
        TriggerWidth::U16 => {
            let lo = u16::from(read_u8(rule.address));
            let hi = u16::from(read_u8(rule.address.wrapping_add(1)));
            u32::from(lo | (hi << 8))
        }
        TriggerWidth::U32 => {
            let b0 = u32::from(read_u8(rule.address));
            let b1 = u32::from(read_u8(rule.address.wrapping_add(1)));
            let b2 = u32::from(read_u8(rule.address.wrapping_add(2)));
            let b3 = u32::from(read_u8(rule.address.wrapping_add(3)));
            b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SplitSource {
    Automatic,
    Manual,
}

#[derive(Debug, Clone, Serialize)]
pub struct SplitEvent {
    pub name: String,
    pub source: SplitSource,
    pub frame: u64,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize)]
pub struct InputLogFrame {
    pub frame: u64,
    pub controller1_bits: u8,
    pub controller2_bits: u8,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone)]
pub enum RtaEvent {
    Started,
    Paused,
    Resumed,
    Split(SplitEvent),
    Invalidated(String),
    Finished(Duration),
}

#[derive(Debug, Clone)]
pub struct RunArtifactPaths {
    pub run_json_path: PathBuf,
    pub input_log_path: Option<PathBuf>,
}

#[derive(Debug, Serialize)]
struct RunArtifact {
    profile_id: String,
    rom_hash: String,
    state: String,
    valid: bool,
    elapsed_ms: u128,
    invalidation_reasons: Vec<String>,
    splits: Vec<SplitEvent>,
}

/// The active state machine that tracks a speedrun session.
///
/// `RtaManager` evaluates memory triggers on every frame, manages the session state
/// (Armed, Running, Finished), and enforces invalidation rules (e.g., forbidding rewinds).
/// It logs splits and can optionally generate a JSON report and input log when a run finishes.
///
/// # Examples
///
/// ```no_run
/// use nes_desktop::rta::{RtaManager, RtaProfile, RtaSessionState};
/// use std::path::PathBuf;
/// use std::time::Instant;
///
/// // Create a basic profile programmatically (usually loaded from TOML).
/// let profile = RtaProfile::default();
///
/// // Initialize the manager with the profile and output directory.
/// let runs_dir = PathBuf::from("runs/rta");
/// let mut manager = RtaManager::new(profile, "rom_hash".to_owned(), runs_dir, None);
///
/// // The manager starts in the `Armed` state.
/// assert_eq!(manager.state(), RtaSessionState::Armed);
///
/// // On every frame, provide the current frame number, the current time,
/// // and a closure that can read memory values for trigger evaluation.
/// let mut memory = [0u8; 0xFFFF];
/// let t0 = Instant::now();
/// let events = manager.tick(1, t0, |addr| memory[usize::from(addr)]);
///
/// // If triggers fire, the manager will return events like `RtaEvent::Started`
/// // and update its internal state accordingly.
/// ```
#[derive(Debug)]
#[doc(alias = "speedrun")]
pub struct RtaManager {
    profile: RtaProfile,
    rom_hash: String,
    state: RtaSessionState,
    start_instant: Option<Instant>,
    pause_started_at: Option<Instant>,
    paused_accumulated: Duration,
    elapsed_at_finish: Option<Duration>,
    finish_frame: Option<u64>,
    invalidation_reasons: BTreeSet<String>,
    split_counter: u64,
    split_events: Vec<SplitEvent>,
    triggers: BTreeMap<TriggerSlot, TriggerRuntime>,
    input_log: Vec<InputLogFrame>,
    runs_dir: PathBuf,
    artifacts_written: Option<RunArtifactPaths>,
    calibration: Option<CalibrationRecorder>,
}

impl RtaManager {
    pub fn new(
        profile: RtaProfile,
        rom_hash: String,
        runs_dir: PathBuf,
        calibration: Option<CalibrationRecorder>,
    ) -> Self {
        let mut triggers = BTreeMap::<TriggerSlot, TriggerRuntime>::new();
        triggers.insert(
            TriggerSlot::Start,
            TriggerRuntime::new(profile.start.clone()),
        );
        triggers.insert(TriggerSlot::End, TriggerRuntime::new(profile.end.clone()));
        if let Some(rule) = profile.pause.clone() {
            triggers.insert(TriggerSlot::Pause, TriggerRuntime::new(rule));
        }
        if let Some(rule) = profile.resume.clone() {
            triggers.insert(TriggerSlot::Resume, TriggerRuntime::new(rule));
        }
        for (idx, split) in profile.splits.iter().enumerate() {
            triggers.insert(
                TriggerSlot::Split(idx),
                TriggerRuntime::new(split.trigger.clone()),
            );
        }

        Self {
            profile,
            rom_hash,
            state: RtaSessionState::Armed,
            start_instant: None,
            pause_started_at: None,
            paused_accumulated: Duration::ZERO,
            elapsed_at_finish: None,
            finish_frame: None,
            invalidation_reasons: BTreeSet::new(),
            split_counter: 0,
            split_events: Vec::new(),
            triggers,
            input_log: Vec::new(),
            runs_dir,
            artifacts_written: None,
            calibration,
        }
    }

    pub fn state(&self) -> RtaSessionState {
        self.state
    }

    pub fn profile_id(&self) -> &str {
        &self.profile.id
    }

    pub fn is_calibrating(&self) -> bool {
        self.calibration.is_some()
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            RtaSessionState::Running | RtaSessionState::InvalidPractice
        )
    }

    pub fn can_manual_override_profile(&self) -> bool {
        matches!(self.state, RtaSessionState::Idle | RtaSessionState::Armed)
    }

    pub fn is_valid_run(&self) -> bool {
        self.invalidation_reasons.is_empty()
    }

    pub fn elapsed(&self, now: Instant) -> Duration {
        if let Some(frozen) = self.elapsed_at_finish {
            return frozen;
        }

        let Some(start) = self.start_instant else {
            return Duration::ZERO;
        };

        let base_end = self.pause_started_at.unwrap_or(now);
        let gross = base_end.saturating_duration_since(start);
        gross.saturating_sub(self.paused_accumulated)
    }

    pub fn split_events(&self) -> &[SplitEvent] {
        &self.split_events
    }

    pub fn invalidation_reasons(&self) -> Vec<String> {
        self.invalidation_reasons.iter().cloned().collect()
    }

    pub fn tick<F>(&mut self, frame: u64, now: Instant, mut read_u8: F) -> Vec<RtaEvent>
    where
        F: FnMut(u16) -> u8,
    {
        if let Some(calibration) = self.calibration.as_mut() {
            calibration.record_frame(frame, &mut read_u8);
        }

        let mut events = Vec::<RtaEvent>::new();

        self.tick_start(now, &mut read_u8, &mut events);

        if !self.is_active() {
            return events;
        }

        let is_paused = self.tick_pause_resume(now, &mut read_u8, &mut events);

        if !is_paused {
            self.tick_splits(frame, now, &mut read_u8, &mut events);
        }

        self.tick_end(frame, now, &mut read_u8, &mut events);

        events
    }

    fn tick_start<F>(&mut self, now: Instant, mut read_u8: F, events: &mut Vec<RtaEvent>)
    where
        F: FnMut(u16) -> u8,
    {
        if self.state == RtaSessionState::Armed
            && self.trigger_fired(TriggerSlot::Start, &mut read_u8)
        {
            self.state = RtaSessionState::Running;
            self.start_instant = Some(now);
            self.pause_started_at = None;
            self.paused_accumulated = Duration::ZERO;
            events.push(RtaEvent::Started);
        }
    }

    fn tick_pause_resume<F>(
        &mut self,
        now: Instant,
        mut read_u8: F,
        events: &mut Vec<RtaEvent>,
    ) -> bool
    where
        F: FnMut(u16) -> u8,
    {
        let is_paused = self.pause_started_at.is_some();
        if is_paused {
            if self.trigger_fired(TriggerSlot::Resume, &mut read_u8)
                && let Some(paused_at) = self.pause_started_at.take()
            {
                self.paused_accumulated = self
                    .paused_accumulated
                    .saturating_add(now.saturating_duration_since(paused_at));
                events.push(RtaEvent::Resumed);
            }
        } else if self.trigger_fired(TriggerSlot::Pause, &mut read_u8) {
            self.pause_started_at = Some(now);
            events.push(RtaEvent::Paused);
        }
        is_paused
    }

    fn tick_splits<F>(
        &mut self,
        frame: u64,
        now: Instant,
        mut read_u8: F,
        events: &mut Vec<RtaEvent>,
    ) where
        F: FnMut(u16) -> u8,
    {
        for idx in 0..self.profile.splits.len() {
            if self.trigger_fired(TriggerSlot::Split(idx), &mut read_u8) {
                let split_name = self.profile.splits[idx].name.clone();
                let event = self.push_split(split_name, SplitSource::Automatic, frame, now);
                events.push(RtaEvent::Split(event));
            }
        }
    }

    fn tick_end<F>(&mut self, frame: u64, now: Instant, mut read_u8: F, events: &mut Vec<RtaEvent>)
    where
        F: FnMut(u16) -> u8,
    {
        if self.trigger_fired(TriggerSlot::End, &mut read_u8) {
            self.state = RtaSessionState::Finished;
            self.finish_frame = Some(frame);
            self.elapsed_at_finish = Some(self.elapsed(now));
            events.push(RtaEvent::Finished(
                self.elapsed_at_finish.unwrap_or_default(),
            ));
        }
    }

    fn trigger_fired<F>(&mut self, slot: TriggerSlot, mut read_u8: F) -> bool
    where
        F: FnMut(u16) -> u8,
    {
        let Some(runtime) = self.triggers.get_mut(&slot) else {
            return false;
        };
        runtime.evaluate(&mut read_u8)
    }

    pub fn mark_forbidden_action(
        &mut self,
        action: ForbiddenAction,
        _frame: u64,
        _now: Instant,
    ) -> Option<RtaEvent> {
        if !self.is_active() {
            return None;
        }
        if !self.profile.invalidation.invalidate_on.contains(&action) {
            return None;
        }

        let reason = action.as_reason().to_owned();
        if !self.invalidation_reasons.insert(reason.clone()) {
            return None;
        }

        if self.state == RtaSessionState::Running {
            self.state = RtaSessionState::InvalidPractice;
        }

        Some(RtaEvent::Invalidated(reason))
    }

    fn push_split(
        &mut self,
        name: String,
        source: SplitSource,
        frame: u64,
        now: Instant,
    ) -> SplitEvent {
        self.split_counter = self.split_counter.saturating_add(1);
        let event = SplitEvent {
            name,
            source,
            frame,
            elapsed_ms: self.elapsed(now).as_millis(),
        };
        self.split_events.push(event.clone());
        event
    }

    pub fn manual_split(&mut self, frame: u64, now: Instant) -> Option<RtaEvent> {
        if !self.is_active() {
            return None;
        }
        let split_name = format!("manual-{}", self.split_counter.saturating_add(1));
        let event = self.push_split(split_name.clone(), SplitSource::Manual, frame, now);
        if let Some(calibration) = self.calibration.as_mut() {
            calibration.mark_split(split_name, frame);
        }
        Some(RtaEvent::Split(event))
    }

    pub fn force_finish(&mut self, frame: u64, now: Instant) -> Option<RtaEvent> {
        if !self.is_active() {
            return None;
        }
        self.state = RtaSessionState::Finished;
        self.finish_frame = Some(frame);
        self.elapsed_at_finish = Some(self.elapsed(now));
        Some(RtaEvent::Finished(
            self.elapsed_at_finish.unwrap_or_default(),
        ))
    }

    pub fn record_input_frame(
        &mut self,
        frame: u64,
        controller1_bits: u8,
        controller2_bits: u8,
        now: Instant,
    ) {
        if !self.profile.logging.save_input_log || !self.is_active() {
            return;
        }
        self.input_log.push(InputLogFrame {
            frame,
            controller1_bits,
            controller2_bits,
            elapsed_ms: self.elapsed(now).as_millis(),
        });
    }

    pub fn write_artifacts_if_finished(&mut self) -> Result<Option<RunArtifactPaths>, String> {
        if self.state != RtaSessionState::Finished {
            return Ok(None);
        }
        if let Some(existing) = self.artifacts_written.clone() {
            return Ok(Some(existing));
        }

        fs::create_dir_all(&self.runs_dir).map_err(|err| {
            format!(
                "failed to create RTA run output directory '{}': {err}",
                self.runs_dir.display()
            )
        })?;

        let stamp = unix_epoch_millis();
        let base_name = format!("{}-{stamp}", sanitize_id_for_filename(&self.profile.id));

        let run_json_path = self.write_run_artifact(&base_name)?;
        let input_log_path = self.write_input_log(&base_name)?;

        let paths = RunArtifactPaths {
            run_json_path,
            input_log_path,
        };
        self.artifacts_written = Some(paths.clone());
        Ok(Some(paths))
    }

    fn write_run_artifact(&self, base_name: &str) -> Result<PathBuf, String> {
        let run_json_path = self.runs_dir.join(format!("{base_name}.run.json"));
        let artifact = RunArtifact {
            profile_id: self.profile.id.clone(),
            rom_hash: self.rom_hash.clone(),
            state: if self.is_valid_run() {
                "finished_valid".to_owned()
            } else {
                "finished_invalid_practice".to_owned()
            },
            valid: self.is_valid_run(),
            elapsed_ms: self.elapsed_at_finish.unwrap_or_default().as_millis(),
            invalidation_reasons: self.invalidation_reasons(),
            splits: self.split_events.clone(),
        };

        let run_json = serde_json::to_string_pretty(&artifact)
            .map_err(|err| format!("failed to serialize RTA run artifact: {err}"))?;
        fs::write(&run_json_path, run_json).map_err(|err| {
            format!(
                "failed to write RTA run artifact '{}': {err}",
                run_json_path.display()
            )
        })?;
        Ok(run_json_path)
    }

    fn write_input_log(&self, base_name: &str) -> Result<Option<PathBuf>, String> {
        if !self.profile.logging.save_input_log {
            return Ok(None);
        }
        let path = self.runs_dir.join(format!("{base_name}.input.json"));
        let json = serde_json::to_string_pretty(&self.input_log)
            .map_err(|err| format!("failed to serialize RTA input log: {err}"))?;
        fs::write(&path, json)
            .map_err(|err| format!("failed to write RTA input log '{}': {err}", path.display()))?;
        Ok(Some(path))
    }

    pub fn write_calibration_draft(
        &self,
        profiles_dir: &Path,
    ) -> Result<Option<DraftOutput>, String> {
        let Some(calibration) = self.calibration.as_ref() else {
            return Ok(None);
        };
        let output = calibration.write_draft_profile(profiles_dir, &self.rom_hash)?;
        Ok(Some(output))
    }
}

fn unix_epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis()
}

fn sanitize_id_for_filename(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for ch in id.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "rta-run".to_owned()
    } else {
        out
    }
}

#[derive(Debug, Clone, Serialize)]
struct DraftCandidate {
    split_name: String,
    address: u16,
    value: u8,
    confidence: f32,
}

#[derive(Debug, Clone, Serialize)]
struct DraftReport {
    profile_id: String,
    source_split_count: usize,
    source_frame_count: usize,
    candidates: Vec<DraftCandidate>,
}

#[derive(Debug, Clone)]
pub struct DraftOutput {
    pub profile_path: PathBuf,
    pub report_path: PathBuf,
}

#[derive(Debug, Clone)]
struct CalibrationFrame {
    frame: u64,
    work_ram: Vec<u8>,
}

#[derive(Debug, Clone)]
struct CalibrationSplitMark {
    name: String,
    frame: u64,
}

#[derive(Debug, Clone)]
pub struct CalibrationRecorder {
    profile_id: String,
    frames: VecDeque<CalibrationFrame>,
    splits: Vec<CalibrationSplitMark>,
    max_frames: usize,
}

impl CalibrationRecorder {
    pub fn new(profile_id: String) -> Self {
        Self {
            profile_id,
            frames: VecDeque::new(),
            splits: Vec::new(),
            max_frames: 30_000,
        }
    }

    /// **Performance optimization:** Uses `VecDeque::pop_front` instead of `Vec::remove(0)`
    /// to avoid an O(N) memory shift of up to 30,000 frames on every single frame execution.
    pub fn record_frame<F>(&mut self, frame: u64, mut read_u8: F)
    where
        F: FnMut(u16) -> u8,
    {
        let mut work_ram = vec![0_u8; 0x0800];
        for (offset, byte) in work_ram.iter_mut().enumerate() {
            *byte = read_u8(offset as u16);
        }
        if self.frames.len() >= self.max_frames {
            self.frames.pop_front();
        }
        self.frames.push_back(CalibrationFrame { frame, work_ram });
    }

    pub fn mark_split(&mut self, name: String, frame: u64) {
        self.splits.push(CalibrationSplitMark { name, frame });
    }

    pub fn write_draft_profile(
        &self,
        profiles_dir: &Path,
        rom_hash: &str,
    ) -> Result<DraftOutput, String> {
        fs::create_dir_all(profiles_dir).map_err(|err| {
            format!(
                "failed to create RTA profiles directory '{}': {err}",
                profiles_dir.display()
            )
        })?;

        let candidates = self.infer_candidates();
        let start_rule = candidates
            .first()
            .map(|candidate| TriggerRule {
                address: candidate.address,
                value: u32::from(candidate.value),
                ..TriggerRule::default()
            })
            .unwrap_or_default();
        let end_rule = candidates
            .last()
            .map(|candidate| TriggerRule {
                address: candidate.address,
                value: u32::from(candidate.value),
                ..TriggerRule::default()
            })
            .unwrap_or_else(|| TriggerRule {
                address: 1,
                ..TriggerRule::default()
            });

        let draft_profile = RtaProfile {
            id: self.profile_id.clone(),
            game: None,
            category: None,
            version: Some("draft-v1".to_owned()),
            status: ProfileStatus::Draft,
            rom_hashes: vec![rom_hash.trim().to_ascii_lowercase()],
            timer: TimerPolicy::default(),
            invalidation: InvalidationPolicy::default(),
            split_policy: SplitPolicy::default(),
            logging: LoggingPolicy::default(),
            start: start_rule,
            pause: None,
            resume: None,
            end: end_rule,
            splits: candidates
                .iter()
                .map(|candidate| SplitRule {
                    name: candidate.split_name.clone(),
                    trigger: TriggerRule {
                        address: candidate.address,
                        value: u32::from(candidate.value),
                        ..TriggerRule::default()
                    },
                })
                .collect(),
        };

        let profile_path = profiles_dir.join(format!("{}.draft.toml", self.profile_id));
        let profile_text = toml::to_string_pretty(&draft_profile)
            .map_err(|err| format!("failed to serialize draft profile: {err}"))?;
        fs::write(&profile_path, profile_text).map_err(|err| {
            format!(
                "failed to write draft profile '{}': {err}",
                profile_path.display()
            )
        })?;

        let report = DraftReport {
            profile_id: self.profile_id.clone(),
            source_split_count: self.splits.len(),
            source_frame_count: self.frames.len(),
            candidates,
        };
        let report_path = profiles_dir.join(format!("{}.draft_report.json", self.profile_id));
        let report_text = serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to serialize draft report: {err}"))?;
        fs::write(&report_path, report_text).map_err(|err| {
            format!(
                "failed to write draft report '{}': {err}",
                report_path.display()
            )
        })?;

        Ok(DraftOutput {
            profile_path,
            report_path,
        })
    }

    fn infer_candidates(&self) -> Vec<DraftCandidate> {
        let mut out = Vec::<DraftCandidate>::new();
        for split in &self.splits {
            let Some(index) = self
                .frames
                .iter()
                .position(|frame| frame.frame == split.frame)
            else {
                continue;
            };
            if index == 0 {
                continue;
            }
            let prev = &self.frames[index - 1].work_ram;
            let curr = &self.frames[index].work_ram;
            let mut selected = None::<(usize, u8, f32)>;
            for address in 0..curr.len() {
                if prev[address] == curr[address] {
                    continue;
                }
                let value = curr[address];
                let stable_after = self
                    .frames
                    .iter()
                    .skip(index + 1)
                    .take(5)
                    .all(|frame| frame.work_ram[address] == value);
                let confidence = if stable_after { 0.95 } else { 0.55 };
                selected = Some((address, value, confidence));
                if stable_after {
                    break;
                }
            }
            if let Some((address, value, confidence)) = selected {
                out.push(DraftCandidate {
                    split_name: split.name.clone(),
                    address: address as u16,
                    value,
                    confidence,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CalibrationRecorder, ForbiddenAction, ProfileSelectionSource, ProfileStatus, RtaManager,
        RtaProfile, RtaSessionState, SplitPolicy, TimerPolicy, TriggerOp, TriggerRule,
        compute_rom_hash, load_profiles, select_profile,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(stem: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("nes-rta-{stem}-{nonce}"));
        fs::create_dir_all(&path).expect("temp dir should be created");
        path
    }

    fn sample_profile_text() -> &'static str {
        r#"
id = "smb-any"
status = "published"
rom_hashes = ["abc123"]

[start]
address = 1
width = "u8"
op = "eq"
value = 7

[end]
address = 2
width = "u8"
op = "eq"
value = 9
"#
    }

    #[test]
    fn profile_parse_defaults_and_trigger_ops_are_supported() {
        let profile = toml::from_str::<RtaProfile>(sample_profile_text())
            .expect("profile should parse from toml");
        assert_eq!(profile.status, ProfileStatus::Published);
        assert_eq!(profile.timer, TimerPolicy::default());
        assert_eq!(profile.split_policy, SplitPolicy::default());

        let changed_rule = toml::from_str::<TriggerRule>(
            r#"
address = 0
width = "u8"
op = "changed"
value = 0
"#,
        )
        .expect("changed op should parse");
        assert_eq!(changed_rule.op, TriggerOp::Changed);
    }

    #[test]
    fn profile_parse_rejects_unknown_fields() {
        let err = toml::from_str::<RtaProfile>(
            r#"
id = "bad"
status = "published"
rom_hashes = ["abc"]

[start]
address = 1
width = "u8"
op = "eq"
value = 1

[end]
address = 2
width = "u8"
op = "eq"
value = 2

unexpected = "boom"
"#,
        )
        .expect_err("unknown field should fail");
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn select_profile_prefers_override_and_handles_draft_rules() {
        let dir = unique_temp_dir("select-profile");
        let published = dir.join("published.toml");
        let draft = dir.join("draft.toml");

        fs::write(
            &published,
            sample_profile_text()
                .replace("abc123", "face00")
                .replace("smb-any", "pub"),
        )
        .expect("published profile write should succeed");

        fs::write(
            &draft,
            sample_profile_text()
                .replace("status = \"published\"", "status = \"draft\"")
                .replace("abc123", "face11")
                .replace("smb-any", "drafty"),
        )
        .expect("draft profile write should succeed");

        let profiles = load_profiles(&dir).expect("profiles should load");

        let override_pick = select_profile(&profiles, "no-match", Some("pub"), false)
            .expect("manual override should pick exact profile");
        assert_eq!(override_pick.source, ProfileSelectionSource::ManualOverride);
        assert_eq!(override_pick.selected.profile.id, "pub");

        let draft_err = select_profile(&profiles, "face11", None, false)
            .expect_err("draft should be rejected in strict mode");
        assert!(draft_err.contains("draft"));

        let auto_pick =
            select_profile(&profiles, "face00", None, false).expect("hash should auto-select");
        assert_eq!(auto_pick.source, ProfileSelectionSource::AutoByRomHash);
        assert_eq!(auto_pick.selected.profile.id, "pub");
    }

    #[test]
    fn state_machine_transitions_and_invalidation_keep_timer_running() {
        let profile = RtaProfile {
            id: "state-test".to_owned(),
            rom_hashes: vec!["hash".to_owned()],
            start: TriggerRule {
                address: 0,
                value: 1,
                ..TriggerRule::default()
            },
            end: TriggerRule {
                address: 1,
                value: 9,
                ..TriggerRule::default()
            },
            ..RtaProfile::default()
        };

        let runs_dir = unique_temp_dir("state-machine");
        let mut manager = RtaManager::new(profile, "hash".to_owned(), runs_dir, None);

        let mut memory = [0_u8; 65_536];
        let t0 = Instant::now();
        let events = manager.tick(1, t0, |addr| memory[usize::from(addr)]);
        assert!(events.is_empty());
        assert_eq!(manager.state(), RtaSessionState::Armed);

        memory[0] = 1;
        let events = manager.tick(2, t0 + Duration::from_millis(10), |addr| {
            memory[usize::from(addr)]
        });
        assert!(
            events
                .iter()
                .any(|event| matches!(event, super::RtaEvent::Started))
        );
        assert_eq!(manager.state(), RtaSessionState::Running);

        let invalidated = manager.mark_forbidden_action(
            ForbiddenAction::Rewind,
            3,
            t0 + Duration::from_millis(20),
        );
        assert!(invalidated.is_some());
        assert_eq!(manager.state(), RtaSessionState::InvalidPractice);

        let elapsed_before_end = manager.elapsed(t0 + Duration::from_millis(50));
        assert!(elapsed_before_end >= Duration::from_millis(40));

        memory[1] = 9;
        let events = manager.tick(4, t0 + Duration::from_millis(60), |addr| {
            memory[usize::from(addr)]
        });
        assert!(
            events
                .iter()
                .any(|event| matches!(event, super::RtaEvent::Finished(_)))
        );
        assert_eq!(manager.state(), RtaSessionState::Finished);
        assert!(!manager.is_valid_run());
    }

    #[test]
    fn artifact_writer_saves_run_json_and_optional_input_log() {
        let profile = RtaProfile {
            id: "artifact-test".to_owned(),
            rom_hashes: vec!["hash".to_owned()],
            logging: super::LoggingPolicy {
                save_input_log: true,
            },
            start: TriggerRule {
                address: 0,
                value: 1,
                ..TriggerRule::default()
            },
            end: TriggerRule {
                address: 1,
                value: 2,
                ..TriggerRule::default()
            },
            ..RtaProfile::default()
        };
        let runs_dir = unique_temp_dir("artifact");
        let mut manager = RtaManager::new(profile, "hash".to_owned(), runs_dir, None);
        let mut memory = [0_u8; 65_536];
        let t0 = Instant::now();

        memory[0] = 1;
        let _ = manager.tick(1, t0, |addr| memory[usize::from(addr)]);
        manager.record_input_frame(1, 0x12, 0x34, t0 + Duration::from_millis(5));
        memory[1] = 2;
        let _ = manager.tick(2, t0 + Duration::from_millis(10), |addr| {
            memory[usize::from(addr)]
        });

        let written = manager
            .write_artifacts_if_finished()
            .expect("artifact write should succeed")
            .expect("finished run should produce artifacts");
        assert!(written.run_json_path.exists());
        assert!(
            written
                .input_log_path
                .as_ref()
                .is_some_and(|path| path.exists())
        );

        let run_json = fs::read_to_string(&written.run_json_path).expect("run json should exist");
        assert!(run_json.contains("artifact-test"));
        assert!(run_json.contains("finished_valid"));
    }

    #[test]
    fn calibration_outputs_draft_profile_and_report() {
        let mut recorder = CalibrationRecorder::new("calibration-smb".to_owned());
        recorder.max_frames = 4; // Explicitly set a low limit to trigger ring buffer eviction logic for code coverage
        let mut memory = [0_u8; 0x800];
        for frame in 0_u64..8 {
            if frame == 3 {
                memory[0x6D] = 0x04;
            }
            if frame == 6 {
                memory[0x75] = 0x01;
            }
            recorder.record_frame(frame, |addr| memory[usize::from(addr)]);
            if frame == 3 {
                recorder.mark_split("1-1".to_owned(), frame);
            }
            if frame == 6 {
                recorder.mark_split("1-2".to_owned(), frame);
            }
        }

        let out_dir = unique_temp_dir("calibration-out");
        let draft = recorder
            .write_draft_profile(&out_dir, "abc123")
            .expect("draft should be written");

        assert!(draft.profile_path.exists());
        assert!(draft.report_path.exists());

        let profile_text =
            fs::read_to_string(&draft.profile_path).expect("draft profile text should be readable");
        assert!(profile_text.contains("status = \"draft\""));
        assert!(profile_text.contains("abc123"));

        let report_text =
            fs::read_to_string(&draft.report_path).expect("draft report should be readable");
        assert!(report_text.contains("calibration-smb"));
        assert!(report_text.contains("source_split_count"));
    }

    #[test]
    fn rom_hash_is_stable_and_lowercase_hex() {
        let hash_a = compute_rom_hash(&[1, 2, 3, 4]);
        let hash_b = compute_rom_hash(&[1, 2, 3, 4]);
        let hash_c = compute_rom_hash(&[1, 2, 3, 5]);

        assert_eq!(hash_a, hash_b);
        assert_ne!(hash_a, hash_c);
        assert_eq!(hash_a.len(), 64);
        assert!(
            hash_a
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
        );
    }

    #[test]
    fn compare_rom_hashes_ignores_case_without_allocation() {
        let a = "abc123DEF";
        let b = "  ABC123def  ";
        let c = "abc123DEX";

        assert!(super::compare_rom_hashes(a, b));
        assert!(!super::compare_rom_hashes(a, c));
    }

    #[test]
    fn select_profile_handles_multiple_matches_and_empty_matches() {
        let dir = unique_temp_dir("select-profile-edge");
        let pub1 = dir.join("pub1.toml");
        let pub2 = dir.join("pub2.toml");

        fs::write(
            &pub1,
            sample_profile_text()
                .replace("abc123", "samehash")
                .replace("smb-any", "pub1"),
        )
        .expect("pub1 write");

        fs::write(
            &pub2,
            sample_profile_text()
                .replace("abc123", "samehash")
                .replace("smb-any", "pub2"),
        )
        .expect("pub2 write");

        let profiles = load_profiles(&dir).expect("profiles should load");

        let empty_err = select_profile(&profiles, "nomatch", None, false)
            .expect_err("should fail with empty matches");
        assert!(empty_err.contains("No RTA profile matched ROM hash nomatch"));
        assert!(empty_err.contains("pub1"));
        assert!(empty_err.contains("pub2"));

        let multi_err = select_profile(&profiles, "samehash", None, false)
            .expect_err("should fail with multiple matches");
        assert!(multi_err.contains("Multiple RTA profiles matched ROM hash samehash"));
        assert!(multi_err.contains("pub1"));
        assert!(multi_err.contains("pub2"));
    }
}
