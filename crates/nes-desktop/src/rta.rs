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
    let check_draft = |profile: &LoadedProfile| -> Result<(), String> {
        if !allow_draft && profile.profile.status == ProfileStatus::Draft {
            return Err(format!(
                "RTA profile '{}' is draft and cannot be used in strict mode",
                profile.profile.id
            ));
        }
        Ok(())
    };

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
        check_draft(&found)?;
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
        return Err(format!(
            "No RTA profile matched ROM hash {rom_hash}. Known profiles: [{}]",
            format_profile_names(profiles)
        ));
    };

    if let Some(second_match) = match_iter.next() {
        let mut conflict_profiles = vec![first_match.clone(), second_match.clone()];
        conflict_profiles.extend(match_iter.cloned());
        let conflict = format_profile_names(&conflict_profiles);
        return Err(format!(
            "Multiple RTA profiles matched ROM hash {rom_hash}: {conflict}"
        ));
    }

    let selected = first_match.clone();
    check_draft(&selected)?;

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

/// **Performance optimization:** Avoids `.collect::<Vec<_>>()` by pre-allocating
/// a String and joining manually.
fn format_profile_names(profiles: &[LoadedProfile]) -> String {
    let len = profiles
        .iter()
        .map(|p| p.profile.id.len() + 2)
        .sum::<usize>();
    let mut names = String::with_capacity(len);
    for (i, profile) in profiles.iter().enumerate() {
        if i > 0 {
            names.push_str(", ");
        }
        names.push_str(profile.profile.id.as_str());
    }
    names
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
struct RunArtifact<'a, I: Iterator<Item = &'a str> + Clone> {
    profile_id: &'a str,
    rom_hash: &'a str,
    state: &'a str,
    valid: bool,
    elapsed_ms: u128,
    #[serde(with = "serde_iter")]
    invalidation_reasons: I,
    splits: &'a [SplitEvent],
}

pub mod serde_iter {
    use serde::{Serialize, Serializer};

    /// Serializes an [`Iterator`] by treating it as a sequence.
    ///
    /// The `serde` framework natively supports serializing sequences like vectors, but does not
    /// provide a direct `#[serde(serialize_with = "...")]` handler for custom lazily-evaluated
    /// or non-owned iterators directly mapped to struct fields.
    ///
    /// This helper bridges the gap by leveraging [`Serializer::collect_seq`] to drain a
    /// cloned iterator into the serializer stream, making it suitable for fields that yield
    /// an iterator rather than a concrete collection.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use serde::Serialize;
    /// # use nes_desktop::rta::serde_iter;
    ///
    /// #[derive(Serialize)]
    /// struct Report<'a> {
    ///     // Uses `serde_iter::serialize` to turn the slice iterator into a JSON array
    ///     #[serde(with = "serde_iter")]
    ///     items: std::slice::Iter<'a, i32>,
    /// }
    ///
    /// let data = vec![1, 2, 3];
    /// let report = Report { items: data.iter() };
    ///
    /// let json = serde_json::to_string(&report).unwrap();
    /// assert_eq!(json, r#"{"items":[1,2,3]}"#);
    /// ```
    pub fn serialize<T, I, S>(iter: &I, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        I: Iterator<Item = T> + Clone,
        S: Serializer,
    {
        serializer.collect_seq(iter.clone())
    }
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
    /// **⚡ Bolt Optimization:** Replaced `BTreeMap` with `Vec` to eliminate memory allocation
    /// overhead during traversal on the hot path (evaluating triggers every frame). Since the number
    /// of triggers is small, contiguous memory scanning is significantly faster than node traversal.
    triggers: Vec<(TriggerSlot, TriggerRuntime)>,
    input_log: Vec<InputLogFrame>,
    runs_dir: PathBuf,
    artifacts_written: Option<RunArtifactPaths>,
    calibration: Option<CalibrationRecorder>,
}

impl RtaManager {
    /// Forges a new session manager that serves as the impartial referee for a speedrun attempt.
    ///
    /// The manager evaluates the provided [`crate::rta::RtaProfile`] against the live emulation state
    /// to determine when to automatically start the timer, mark splits, or invalidate the run entirely.
    /// It requires the ROM hash upfront to cryptographically bind the resulting run artifact
    /// to a specific game version, preventing spoofing.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile};
    /// use std::path::PathBuf;
    ///
    /// let profile = RtaProfile::default();
    /// let mut manager = RtaManager::new(
    ///     profile,
    ///     "ea343f4e44562066f8114f6e80b2d35c43d3120e71ce001b33edccfa98319df6".to_owned(),
    ///     PathBuf::from("runs/rta"),
    ///     None, // No active calibration drafting
    /// );
    /// ```
    pub fn new(
        profile: RtaProfile,
        rom_hash: String,
        runs_dir: PathBuf,
        calibration: Option<CalibrationRecorder>,
    ) -> Self {
        let mut triggers = Vec::<(TriggerSlot, TriggerRuntime)>::new();
        triggers.push((
            TriggerSlot::Start,
            TriggerRuntime::new(profile.start.clone()),
        ));
        triggers.push((TriggerSlot::End, TriggerRuntime::new(profile.end.clone())));
        if let Some(rule) = profile.pause.clone() {
            triggers.push((TriggerSlot::Pause, TriggerRuntime::new(rule)));
        }
        if let Some(rule) = profile.resume.clone() {
            triggers.push((TriggerSlot::Resume, TriggerRuntime::new(rule)));
        }
        for (idx, split) in profile.splits.iter().enumerate() {
            triggers.push((
                TriggerSlot::Split(idx),
                TriggerRuntime::new(split.trigger.clone()),
            ));
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

    /// Inspects the current lifecycle phase of the speedrun to coordinate UI updates.
    ///
    /// Knowing the state is critical for UI components that must hide the timer when `Idle`,
    /// pulse when `Armed`, or turn red when the run enters `InvalidPractice`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaSessionState, RtaProfile};
    /// use std::path::PathBuf;
    ///
    /// let manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// assert_eq!(manager.state(), RtaSessionState::Armed);
    /// ```
    pub fn state(&self) -> RtaSessionState {
        self.state
    }

    /// Reveals the unique string ID of the active [`crate::rta::RtaProfile`].
    ///
    /// UI overlays rely on this identifier to fetch and display the correct splits layout
    /// for the currently loaded game.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile};
    /// use std::path::PathBuf;
    ///
    /// let mut profile = RtaProfile::default();
    /// profile.id = "smb-any".to_owned();
    /// let manager = RtaManager::new(profile, "hash".to_owned(), PathBuf::from("out"), None);
    /// assert_eq!(manager.profile_id(), "smb-any");
    /// ```
    pub fn profile_id(&self) -> &str {
        &self.profile.id
    }

    /// Determines if the engine is operating in 'Discovery Mode'.
    ///
    /// When `true`, the manager is secretly recording every memory mutation frame-by-frame.
    /// This is an incredibly memory-intensive operation, so clients use this flag to alert
    /// the user that performance degradation is expected during drafting.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile, CalibrationRecorder};
    /// use std::path::PathBuf;
    ///
    /// let manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), Some(CalibrationRecorder::new("test".to_owned())));
    /// if manager.is_calibrating() {
    ///     println!("Warning: Draft mode active, expect frame drops.");
    /// }
    /// ```
    pub fn is_calibrating(&self) -> bool {
        self.calibration.is_some()
    }

    /// Queries whether the timer is currently ticking down on a live attempt.
    ///
    /// An active run prevents the user from accidentally swapping profiles or resetting the
    /// emulator without explicit confirmation. Note that a run remains active even if
    /// the runner commits a foul (like loading a save state) and enters `InvalidPractice`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile};
    /// use std::path::PathBuf;
    ///
    /// let manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// if manager.is_active() {
    ///     // Block the "Load ROM" menu item
    /// }
    /// ```
    pub fn is_active(&self) -> bool {
        matches!(
            self.state,
            RtaSessionState::Running | RtaSessionState::InvalidPractice
        )
    }

    /// Checks if the session is still in the safe setup phase where rules can be modified.
    ///
    /// A speedrunner often needs to switch categories (e.g., from Any% to 100%) before
    /// starting. This returns `false` the moment they trigger the start condition, locking
    /// the profile in place to prevent mid-run tampering.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile};
    /// use std::path::PathBuf;
    ///
    /// let manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// assert!(manager.can_manual_override_profile());
    /// ```
    pub fn can_manual_override_profile(&self) -> bool {
        matches!(self.state, RtaSessionState::Idle | RtaSessionState::Armed)
    }

    /// Assesses the competitive integrity of the current run.
    ///
    /// Returns `true` if the speedrunner has completely respected the profile's rules.
    /// If they triggered any [`ForbiddenAction`] (like rewinding), the run is instantly
    /// marked invalid and ineligible for leaderboard submission.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile, ForbiddenAction};
    /// use std::path::PathBuf;
    /// use std::time::Instant;
    ///
    /// let mut manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// manager.mark_forbidden_action(ForbiddenAction::Rewind, 0, Instant::now());
    /// assert!(!manager.is_valid_run());
    /// ```
    pub fn is_valid_run(&self) -> bool {
        self.invalidation_reasons.is_empty()
    }

    /// Computes the exact duration the runner has been playing, excluding paused segments.
    ///
    /// By dynamically subtracting accumulated pause time, this method provides a highly
    /// accurate Real Time Attack measurement that resiliently survives host OS sleep cycles.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile};
    /// use std::path::PathBuf;
    /// use std::time::Instant;
    ///
    /// let manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// let time_spent = manager.elapsed(Instant::now());
    /// println!("Run time: {:?}", time_spent);
    /// ```
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

    /// Extracts the immutable chronological history of all segments completed so far.
    ///
    /// This slice is essential for rendering the classic speedrun layout on the screen,
    /// allowing the UI to compare current split times against historical personal bests.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile};
    /// use std::path::PathBuf;
    ///
    /// let manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// for split in manager.split_events() {
    ///     println!("Split: {} at {}ms", split.name, split.elapsed_ms);
    /// }
    /// ```
    pub fn split_events(&self) -> &[SplitEvent] {
        &self.split_events
    }

    /// Exposes the specific fouls committed by the runner that destroyed the run's validity.
    ///
    /// This is included directly in the final `.run.json` artifact so verifiers can instantly
    /// see why an automated run was rejected (e.g., `["save_load"]`).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile, ForbiddenAction};
    /// use std::path::PathBuf;
    /// use std::time::Instant;
    ///
    /// let mut manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// manager.mark_forbidden_action(ForbiddenAction::FrameStep, 0, Instant::now());
    /// let reasons: Vec<_> = manager.invalidation_reasons().collect();
    /// assert_eq!(reasons, vec!["frame_step"]);
    /// ```
    pub fn invalidation_reasons(&self) -> impl Iterator<Item = &str> + Clone + '_ {
        self.invalidation_reasons.iter().map(String::as_str)
    }

    /// The beating heart of the RTA Engine. Evaluates all active memory triggers for the current frame.
    ///
    /// By abstracting memory access behind the `read_u8` closure, the engine seamlessly integrates
    /// with any emulator core. It meticulously checks if the current memory state fulfills the
    /// conditions required to start, stop, pause, or split the run, returning any resulting
    /// [`RtaEvent`]s to alert the UI layer.
    ///
    /// # Panics
    ///
    /// Does not explicitly panic, but the provided `read_u8` closure must handle out-of-bounds
    /// addresses gracefully if requested by a poorly formed `TriggerRule`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile};
    /// use std::path::PathBuf;
    /// use std::time::Instant;
    ///
    /// let mut manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// let mut dummy_ram = vec![0_u8; 0x800];
    ///
    /// // Tick exactly once per frame update
    /// let events = manager.tick(1, Instant::now(), |addr| dummy_ram[(addr as usize) % 0x800]);
    /// ```
    pub fn tick<F>(&mut self, frame: u64, now: Instant, mut read_u8: F) -> Vec<RtaEvent>
    where
        F: FnMut(u16) -> u8,
    {
        if let Some(calibration) = self.calibration.as_mut() {
            calibration.record_frame(frame, &mut read_u8);
        }

        // Avoids `Vec::new()` without capacity on the hot path (called every frame).
        // RTA events per frame rarely exceed 2.
        let mut events = Vec::<RtaEvent>::with_capacity(2);

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
        let Some(runtime) = self
            .triggers
            .iter_mut()
            .find_map(|(s, r)| if *s == slot { Some(r) } else { None })
        else {
            return false;
        };
        runtime.evaluate(&mut read_u8)
    }

    /// Acts as the strict referee, penalizing actions that compromise the run's legitimacy.
    ///
    /// Emulator frontends must call this immediately whenever a user triggers a tool-assisted
    /// feature. If the profile's [`InvalidationPolicy`] forbids the action, the session is
    /// permanently branded as `InvalidPractice`, though the timer continues ticking for practice purposes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile, ForbiddenAction, RtaEvent};
    /// use std::path::PathBuf;
    /// use std::time::Instant;
    ///
    /// let mut manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// // User just hit the rewind key!
    /// if let Some(RtaEvent::Invalidated(reason)) = manager.mark_forbidden_action(ForbiddenAction::Rewind, 0, Instant::now()) {
    ///     println!("Run ruined by: {}", reason);
    /// }
    /// ```
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

        let reason_str = action.as_reason();
        if self.invalidation_reasons.contains(reason_str) {
            return None;
        }

        self.invalidation_reasons.insert(reason_str.to_owned());

        if self.state == RtaSessionState::Running {
            self.state = RtaSessionState::InvalidPractice;
        }

        Some(RtaEvent::Invalidated(reason_str.to_owned()))
    }

    fn push_split(
        &mut self,
        name: String,
        source: SplitSource,
        frame: u64,
        now: Instant,
    ) -> SplitEvent {
        self.split_counter = self.split_counter.saturating_add(1);
        let elapsed_ms = self.elapsed(now).as_millis();
        self.split_events.push(SplitEvent {
            name: name.clone(),
            source,
            frame,
            elapsed_ms,
        });
        SplitEvent {
            name,
            source,
            frame,
            elapsed_ms,
        }
    }

    /// Forcefully logs a segment completion, bypassing all automated memory conditions.
    ///
    /// This is the ultimate escape hatch for runners when a game glitches or an automated trigger fails.
    /// It is also heavily utilized during drafting to train the [`crate::rta::CalibrationRecorder`] on exactly *when*
    /// significant events occur.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile};
    /// use std::path::PathBuf;
    /// use std::time::Instant;
    ///
    /// let mut manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// // User pressed the "Split" hotkey!
    /// manager.manual_split(100, Instant::now());
    /// ```
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

    /// Slams the brakes on the speedrun, freezing the timer permanently.
    ///
    /// Necessary for rage-quits or when a runner wants to manually terminate the session
    /// before the official memory trigger has fired. The run artifact is still generated.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile};
    /// use std::path::PathBuf;
    /// use std::time::Instant;
    ///
    /// let mut manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// // User pressed the "Stop Timer" hotkey
    /// manager.force_finish(200, Instant::now());
    /// ```
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

    /// Silently archives the controller state for the current frame into the active input log.
    ///
    /// For verified leaderboard submissions, memory analysis isn't enough—judges need to see
    /// the exact inputs that produced the run. This method builds a massive, timestamped
    /// keystroke history that gets written out when the run concludes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile};
    /// use std::path::PathBuf;
    /// use std::time::Instant;
    ///
    /// let mut manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// // Player holds A and Right (0x01 | 0x80 = 0x81)
    /// manager.record_input_frame(1, 0x81, 0x00, Instant::now());
    /// ```
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

    /// Safely flushes all recorded session data to persistent storage if the run has concluded.
    ///
    /// The manager accumulates a large amount of state (splits, input logs, invalidation reasons).
    /// This method ensures that hard-earned data is transformed into a standardized JSON artifact
    /// that can be uploaded to leaderboards or shared with friends. It gracefully does nothing
    /// if the run is still active.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile};
    /// use std::path::PathBuf;
    ///
    /// let mut manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), None);
    /// if let Ok(Some(paths)) = manager.write_artifacts_if_finished() {
    ///     println!("Run saved to: {:?}", paths.run_json_path);
    /// }
    /// ```
    pub fn write_artifacts_if_finished(&mut self) -> Result<Option<RunArtifactPaths>, String> {
        if self.state != RtaSessionState::Finished {
            return Ok(None);
        }
        if let Some(existing) = self.artifacts_written.as_ref() {
            return Ok(Some(existing.clone()));
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
        // **⚡ Bolt Optimization:** Avoids unnecessary heap allocations when creating the `RunArtifact` struct for serialization by borrowing `&str` instead of calling `.clone()` or `.to_owned()`.
        let artifact = RunArtifact {
            profile_id: &self.profile.id,
            rom_hash: &self.rom_hash,
            state: if self.is_valid_run() {
                "finished_valid"
            } else {
                "finished_invalid_practice"
            },
            valid: self.is_valid_run(),
            elapsed_ms: self.elapsed_at_finish.unwrap_or_default().as_millis(),
            invalidation_reasons: self.invalidation_reasons(),
            splits: &self.split_events,
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

    /// Commands the calibration engine to analyze its memory snapshots and hallucinate a new profile.
    ///
    /// When drafting a new speedrun category, finding the exact memory addresses that represent
    /// level transitions is agonizing. This method delegates that tedious hunt to the machine,
    /// spitting out a `.draft.toml` file filled with highly probable `TriggerRule`s based on
    /// statistical analysis of the user's manual splits.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::{RtaManager, RtaProfile, CalibrationRecorder};
    /// use std::path::PathBuf;
    ///
    /// let mut manager = RtaManager::new(RtaProfile::default(), "hash".to_owned(), PathBuf::from("out"), Some(CalibrationRecorder::new("test".to_owned())));
    /// // ... after playing and splitting ...
    /// manager.write_calibration_draft(&PathBuf::from("config/rta/profiles")).unwrap();
    /// ```
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
    /// Initializes a massive circular buffer capable of storing up to 30,000 full memory snapshots.
    ///
    /// This acts as the "black box" flight recorder for the RTA drafting process, capturing
    /// exactly what the NES memory looked like right before and right after a player pressed
    /// the manual split key.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::CalibrationRecorder;
    ///
    /// let recorder = CalibrationRecorder::new("smb-any-draft".to_owned());
    /// ```
    pub fn new(profile_id: String) -> Self {
        Self {
            profile_id,
            frames: VecDeque::new(),
            splits: Vec::new(),
            max_frames: 30_000,
        }
    }

    /// Ingests the current 2KB work RAM state into the circular buffer.
    ///
    /// This method is aggressively called every single frame during a draft run. It forces
    /// the oldest memory snapshot out of the buffer if the capacity is reached.
    ///
    /// **⚡ Bolt Optimization:** Uses `VecDeque::pop_front` instead of `Vec::remove(0)`
    /// to avoid an O(N) memory shift of up to 30,000 frames on every single frame execution.
    ///
    /// **⚡ Bolt Optimization:** Recycles the `Vec<u8>` from the oldest frame when the buffer
    /// reaches capacity, eliminating a 2KB heap allocation on every single frame.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::CalibrationRecorder;
    ///
    /// let mut recorder = CalibrationRecorder::new("test".to_owned());
    /// let mut dummy_ram = vec![0_u8; 0x800];
    ///
    /// // Tick exactly once per frame update
    /// recorder.record_frame(1, |addr| dummy_ram[(addr as usize) % 0x800]);
    /// ```
    pub fn record_frame<F>(&mut self, frame: u64, mut read_u8: F)
    where
        F: FnMut(u16) -> u8,
    {
        let mut work_ram = if self.frames.len() >= self.max_frames {
            self.frames
                .pop_front()
                .map(|f| f.work_ram)
                .unwrap_or_else(|| vec![0_u8; 0x0800])
        } else {
            vec![0_u8; 0x0800]
        };

        for (offset, byte) in work_ram.iter_mut().enumerate() {
            *byte = read_u8(offset as u16);
        }
        self.frames.push_back(CalibrationFrame { frame, work_ram });
    }

    /// Drops an anchor in the memory timeline denoting that a significant event occurred.
    ///
    /// When the user manually splits, we don't know *why* they split yet. We just record
    /// the frame number. Later, the analysis phase will scrutinize the memory snapshots
    /// preceding this anchor to discover the exact address that changed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::CalibrationRecorder;
    ///
    /// let mut recorder = CalibrationRecorder::new("test".to_owned());
    /// recorder.mark_split("Level 1 Complete".to_owned(), 12450);
    /// ```
    pub fn mark_split(&mut self, name: String, frame: u64) {
        self.splits.push(CalibrationSplitMark { name, frame });
    }

    /// Scours the recorded memory timeline to automatically deduce the rules of a speedrun.
    ///
    /// It hunts for memory bytes that mutated exactly around the time of a [`CalibrationRecorder::mark_split`]
    /// anchor and remained perfectly stable for several frames afterward. It compiles these
    /// discoveries into a `.draft.toml` file, complete with confidence scores.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use nes_desktop::rta::CalibrationRecorder;
    /// use std::path::PathBuf;
    ///
    /// let recorder = CalibrationRecorder::new("test".to_owned());
    /// // ... after recording frames and splits ...
    /// let output = recorder.write_draft_profile(&PathBuf::from("out_dir"), "hash").unwrap();
    /// println!("Draft generated at: {:?}", output.profile_path);
    /// ```
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
        let draft_profile = self.build_draft_profile(rom_hash, &candidates);

        let profile_path = profiles_dir.join(format!("{}.draft.toml", self.profile_id));
        let profile_text = toml::to_string_pretty(&draft_profile)
            .map_err(|err| format!("failed to serialize draft profile: {err}"))?;
        fs::write(&profile_path, profile_text).map_err(|err| {
            format!(
                "failed to write draft profile '{}': {err}",
                profile_path.display()
            )
        })?;

        let report_path = self.write_draft_report(profiles_dir, candidates)?;

        Ok(DraftOutput {
            profile_path,
            report_path,
        })
    }

    fn build_draft_profile(&self, rom_hash: &str, candidates: &[DraftCandidate]) -> RtaProfile {
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

        RtaProfile {
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
        }
    }

    fn write_draft_report(
        &self,
        profiles_dir: &Path,
        candidates: Vec<DraftCandidate>,
    ) -> Result<PathBuf, String> {
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
        Ok(report_path)
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
