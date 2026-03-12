# nes-ai Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a new `nes-ai` crate that can train a Burn-based control policy for Super Mario Bros. World 1-1 from a fixed gameplay snapshot, while keeping the crate structured around reusable profile seams for future games.

**Architecture:** Add `crates/nes-ai` as a workspace member and keep the emulator boundary hard: `nes-core` remains the deterministic runtime, while `nes-ai` owns profiles, observation preprocessing, reward logic, episode artifacts, Burn models, and PPO training. Use a generic profile-driven environment internally, then expose an `SmbControlEnv` constructor for the first milestone so future games can reuse the same trainer without SMB logic leaking everywhere.

**Tech Stack:** Rust 2024, `nes-core` with `tas`, `serde`, `serde_json`, `toml`, `sha2`, `thiserror`, Burn `0.20.1` with `train` + `ndarray`, cargo test.

---

**Execution Notes**

- Required execution skills: `@test-driven-development`, `@verification-before-completion`, `@rust-router`, `@domain-ml`.
- Do not commit a generated SMB save-state snapshot to the repository. `CoreSnapshot` contains mapper state, which would effectively package ROM-derived data into the repo.
- Keep all unignored tests ROM-free by using synthetic snapshots, loop cores, or test-only mock profiles. SMB-specific training/evaluation tests should be `#[ignore]` and consume locally generated artifacts.

### Task 1: Scaffold The `nes-ai` Crate And Action Space

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/nes-ai/Cargo.toml`
- Create: `crates/nes-ai/src/lib.rs`
- Create: `crates/nes-ai/src/actions.rs`
- Create: `crates/nes-ai/src/error.rs`
- Test: `crates/nes-ai/tests/action_space.rs`

**Step 1: Write the failing test**

Create `crates/nes-ai/tests/action_space.rs`:

```rust
use nes_ai::actions::ControlAction;
use nes_core::Button;

#[test]
fn action_ids_map_to_expected_controller_masks() {
    assert_eq!(ControlAction::Noop.controller1_bits(), 0);
    assert_eq!(ControlAction::Right.controller1_bits(), Button::Right.bit_mask());
    assert_eq!(
        ControlAction::RightA.controller1_bits(),
        Button::Right.bit_mask() | Button::A.bit_mask()
    );
    assert_eq!(ControlAction::A.controller1_bits(), Button::A.bit_mask());
    assert_eq!(
        ControlAction::RightB.controller1_bits(),
        Button::Right.bit_mask() | Button::B.bit_mask()
    );
    assert_eq!(
        ControlAction::RightAB.controller1_bits(),
        Button::Right.bit_mask() | Button::A.bit_mask() | Button::B.bit_mask()
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p nes-ai action_ids_map_to_expected_controller_masks -- --exact`

Expected: FAIL because the workspace does not yet contain a `nes-ai` package.

**Step 3: Write minimal implementation**

Update the workspace root `Cargo.toml`:

```toml
members = [
    "crates/nes-core",
    "crates/nes-dsl",
    "crates/nes-config",
    "crates/nes-mcp",
    "crates/nes-desktop",
    "crates/nes-tui",
    "crates/nes-web",
    "crates/nes-proof",
    "crates/nes-test-harness",
    "crates/nes-netplay",
    "crates/nes-relay",
    "crates/nes-rewind",
    "crates/nes-ai",
]
```

Create `crates/nes-ai/Cargo.toml`:

```toml
[package]
name = "nes-ai"
version = "0.1.0"
edition = "2024"
rust-version = "1.85"

[lints.rust]
unsafe_code = "warn"

[lints.clippy]
all = "warn"
pedantic = "warn"

[dependencies]
burn = { version = "0.20.1", default-features = false, features = ["std", "train", "ndarray"] }
nes-core = { path = "../nes-core", features = ["tas"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
thiserror = "2"
toml = "0.8"

[dev-dependencies]
tempfile = "3"
```

Create `crates/nes-ai/src/lib.rs`:

```rust
pub mod actions;
pub mod error;
```

Create `crates/nes-ai/src/actions.rs`:

```rust
use nes_core::Button;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    Noop,
    Right,
    RightA,
    A,
    RightB,
    RightAB,
}

impl ControlAction {
    #[must_use]
    pub const fn action_count() -> usize {
        6
    }

    #[must_use]
    pub fn controller1_bits(self) -> u8 {
        match self {
            Self::Noop => 0,
            Self::Right => Button::Right.bit_mask(),
            Self::RightA => Button::Right.bit_mask() | Button::A.bit_mask(),
            Self::A => Button::A.bit_mask(),
            Self::RightB => Button::Right.bit_mask() | Button::B.bit_mask(),
            Self::RightAB => Button::Right.bit_mask() | Button::A.bit_mask() | Button::B.bit_mask(),
        }
    }
}
```

Create `crates/nes-ai/src/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p nes-ai action_ids_map_to_expected_controller_masks -- --exact`

Expected: PASS

**Step 5: Commit**

```bash
git add Cargo.toml crates/nes-ai/Cargo.toml crates/nes-ai/src/lib.rs crates/nes-ai/src/actions.rs crates/nes-ai/src/error.rs crates/nes-ai/tests/action_space.rs
git commit -m "feat(nes-ai): scaffold crate and control action space"
```

### Task 2: Add Snapshot Bundle IO And Local SMB Snapshot Preparation

**Files:**
- Modify: `crates/nes-ai/src/lib.rs`
- Modify: `crates/nes-ai/src/error.rs`
- Create: `crates/nes-ai/src/snapshot.rs`
- Create: `crates/nes-ai/src/bin/prepare_smb_control.rs`
- Create: `crates/nes-ai/assets/bootstrap/smb_1_1_entry.tas.json`
- Create: `config/ai/profiles/smb-control.example.toml`
- Test: `crates/nes-ai/tests/snapshot_bundle.rs`

**Step 1: Write the failing test**

Create `crates/nes-ai/tests/snapshot_bundle.rs`:

```rust
use nes_ai::snapshot::{load_snapshot_bundle, write_snapshot_bundle};
use nes_core::NesCore;
use tempfile::tempdir;

#[test]
fn snapshot_bundle_round_trip_preserves_rom_hash_and_snapshot() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("control.state.json");

    let snapshot = NesCore::new().save_state();
    write_snapshot_bundle(&path, "rom-hash", "smb-control-v1", &snapshot).unwrap();

    let bundle = load_snapshot_bundle(&path).unwrap();
    assert_eq!(bundle.rom_hash, "rom-hash");
    assert_eq!(bundle.snapshot, snapshot);
    assert_eq!(bundle.snapshot_id, "smb-control-v1");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p nes-ai snapshot_bundle_round_trip_preserves_rom_hash_and_snapshot -- --exact`

Expected: FAIL because `nes_ai::snapshot` does not exist yet.

**Step 3: Write minimal implementation**

Create `crates/nes-ai/src/snapshot.rs`:

```rust
use std::fs;
use std::path::Path;

use nes_core::CoreSnapshot;
use serde::{Deserialize, Serialize};

use crate::error::AiError;

pub const SNAPSHOT_BUNDLE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotBundle {
    pub version: u32,
    pub rom_hash: String,
    pub snapshot_id: String,
    pub snapshot: CoreSnapshot,
}

pub fn write_snapshot_bundle(
    path: &Path,
    rom_hash: &str,
    snapshot_id: &str,
    snapshot: &CoreSnapshot,
) -> Result<(), AiError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| AiError::Unsupported("snapshot dir create"))?;
    }
    let bundle = SnapshotBundle {
        version: SNAPSHOT_BUNDLE_VERSION,
        rom_hash: rom_hash.to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        snapshot: snapshot.clone(),
    };
    let json = serde_json::to_vec_pretty(&bundle)
        .map_err(|_| AiError::Unsupported("snapshot serialize"))?;
    fs::write(path, json).map_err(|_| AiError::Unsupported("snapshot write"))?;
    Ok(())
}

pub fn load_snapshot_bundle(path: &Path) -> Result<SnapshotBundle, AiError> {
    let bytes = fs::read(path).map_err(|_| AiError::Unsupported("snapshot read"))?;
    let bundle: SnapshotBundle =
        serde_json::from_slice(&bytes).map_err(|_| AiError::Unsupported("snapshot parse"))?;
    if bundle.version != SNAPSHOT_BUNDLE_VERSION {
        return Err(AiError::Unsupported("snapshot version"));
    }
    Ok(bundle)
}
```

Extend `crates/nes-ai/src/error.rs`:

```rust
#[derive(Debug, Error)]
pub enum AiError {
    #[error("unsupported operation: {0}")]
    Unsupported(&'static str),
    #[error("ROM hash mismatch: expected {expected}, found {found}")]
    RomHashMismatch { expected: String, found: String },
}
```

Update `crates/nes-ai/src/lib.rs`:

```rust
pub mod actions;
pub mod error;
pub mod snapshot;
```

Create a committed bootstrap TAS asset at `crates/nes-ai/assets/bootstrap/smb_1_1_entry.tas.json` using `TasMovie` JSON so the repo stores only controller input, not ROM-derived state:

```json
{"runs":[{"controller1_bits":0,"controller2_bits":0,"frames":180},{"controller1_bits":8,"controller2_bits":0,"frames":4},{"controller1_bits":0,"controller2_bits":0,"frames":90}]}
```

Create `crates/nes-ai/src/bin/prepare_smb_control.rs` with a narrow CLI:

```rust
use std::{env, fs, path::PathBuf};

use sha2::{Digest, Sha256};

use nes_ai::snapshot::write_snapshot_bundle;
use nes_core::{NesCore, tas::TasMovie};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        eprintln!("Usage: prepare_smb_control <rom_path> <bootstrap_tas_json> <output_snapshot>");
        std::process::exit(2);
    }

    let rom_path = PathBuf::from(&args[1]);
    let movie_path = PathBuf::from(&args[2]);
    let out_path = PathBuf::from(&args[3]);

    let rom = fs::read(&rom_path)?;
    let rom_hash = format!("{:x}", Sha256::digest(&rom));
    let movie: TasMovie = serde_json::from_slice(&fs::read(movie_path)?)?;

    let mut core = NesCore::new();
    core.load_ines_rom(&rom)?;
    movie.replay(&mut core)?;

    write_snapshot_bundle(&out_path, &rom_hash, "smb-control-v1", &core.save_state())?;
    println!("{}", out_path.display());
    Ok(())
}
```

Create `config/ai/profiles/smb-control.example.toml`:

```toml
id = "smb-control"
rom_path = "C:/path/to/Super Mario Bros.nes"
snapshot_path = "artifacts/ai/snapshots/smb-1-1-control.state.json"
bootstrap_tas_path = "crates/nes-ai/assets/bootstrap/smb_1_1_entry.tas.json"
frame_stack = 4
frame_skip = 4
max_episode_frames = 900
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p nes-ai snapshot_bundle_round_trip_preserves_rom_hash_and_snapshot -- --exact`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/nes-ai/src/lib.rs crates/nes-ai/src/error.rs crates/nes-ai/src/snapshot.rs crates/nes-ai/src/bin/prepare_smb_control.rs crates/nes-ai/assets/bootstrap/smb_1_1_entry.tas.json config/ai/profiles/smb-control.example.toml crates/nes-ai/tests/snapshot_bundle.rs
git commit -m "feat(nes-ai): add snapshot bundle io and smb snapshot prep"
```

### Task 3: Implement Profile Config And Observation Preprocessing

**Files:**
- Modify: `crates/nes-ai/src/lib.rs`
- Create: `crates/nes-ai/src/config.rs`
- Create: `crates/nes-ai/src/profile.rs`
- Create: `crates/nes-ai/src/obs.rs`
- Create: `crates/nes-ai/src/profiles/mod.rs`
- Create: `crates/nes-ai/src/profiles/smb.rs`
- Test: `crates/nes-ai/tests/profile_config.rs`
- Test: `crates/nes-ai/tests/observation_contract.rs`

**Step 1: Write the failing tests**

Create `crates/nes-ai/tests/profile_config.rs`:

```rust
use nes_ai::config::AiProfileConfig;

#[test]
fn profile_config_parses_expected_control_defaults() {
    let cfg: AiProfileConfig = toml::from_str(
        r#"
id = "smb-control"
rom_path = "roms/smb.nes"
snapshot_path = "artifacts/smb.state.json"
bootstrap_tas_path = "crates/nes-ai/assets/bootstrap/smb_1_1_entry.tas.json"
frame_stack = 4
frame_skip = 4
max_episode_frames = 900

[observation]
width = 84
height = 84

[reward]
forward_progress = 1.0
alive_bonus = 0.01
stall_penalty = -0.02
death_penalty = -1.0
stall_frames = 120
"#,
    )
    .unwrap();

    assert_eq!(cfg.frame_stack, 4);
    assert_eq!(cfg.observation.width, 84);
    assert_eq!(cfg.reward.stall_frames, 120);
}
```

Create `crates/nes-ai/tests/observation_contract.rs`:

```rust
use nes_ai::obs::{downsample_grayscale, FrameStack};
use nes_core::{FRAME_HEIGHT, FRAME_WIDTH};

#[test]
fn grayscale_downsample_outputs_expected_shape_and_unit_range() {
    let mut rgba = vec![0_u8; FRAME_WIDTH * FRAME_HEIGHT * 4];
    rgba[0] = 255;
    rgba[1] = 255;
    rgba[2] = 255;
    rgba[3] = 255;

    let image = downsample_grayscale(&rgba, 84, 84);
    assert_eq!(image.len(), 84 * 84);
    assert!(image.iter().all(|value| (0.0..=1.0).contains(value)));
}

#[test]
fn frame_stack_retains_only_recent_frames() {
    let mut stack = FrameStack::new(2, 4);
    stack.push(vec![0.0; 4]);
    stack.push(vec![1.0; 4]);
    stack.push(vec![2.0; 4]);

    let frames = stack.as_slices();
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0], &[1.0, 1.0, 1.0, 1.0]);
    assert_eq!(frames[1], &[2.0, 2.0, 2.0, 2.0]);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p nes-ai profile_config_parses_expected_control_defaults -- --exact`

Expected: FAIL because `config` does not exist.

Run: `cargo test -p nes-ai grayscale_downsample_outputs_expected_shape_and_unit_range -- --exact`

Expected: FAIL because `obs` does not exist.

**Step 3: Write minimal implementation**

Create `crates/nes-ai/src/config.rs`:

```rust
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiProfileConfig {
    pub id: String,
    pub rom_path: PathBuf,
    pub snapshot_path: PathBuf,
    pub bootstrap_tas_path: PathBuf,
    pub frame_stack: usize,
    pub frame_skip: u32,
    pub max_episode_frames: u32,
    pub observation: ObservationConfig,
    pub reward: RewardConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationConfig {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RewardConfig {
    pub forward_progress: f32,
    pub alive_bonus: f32,
    pub stall_penalty: f32,
    pub death_penalty: f32,
    pub stall_frames: u32,
}
```

Create `crates/nes-ai/src/profile.rs`:

```rust
use nes_core::NesCore;

use crate::config::AiProfileConfig;

pub trait TaskProfile {
    type Features: Clone + PartialEq + core::fmt::Debug;

    fn config(&self) -> &AiProfileConfig;
    fn decode_features(&self, core: &NesCore) -> Self::Features;
}
```

Create `crates/nes-ai/src/obs.rs`:

```rust
use std::collections::VecDeque;

pub fn downsample_grayscale(rgba: &[u8], width: usize, height: usize) -> Vec<f32> {
    let src_width = 256;
    let src_height = 240;
    let mut out = vec![0.0; width * height];
    for y in 0..height {
        for x in 0..width {
            let src_x = x * src_width / width;
            let src_y = y * src_height / height;
            let idx = (src_y * src_width + src_x) * 4;
            let r = rgba[idx] as f32;
            let g = rgba[idx + 1] as f32;
            let b = rgba[idx + 2] as f32;
            out[y * width + x] = (0.299 * r + 0.587 * g + 0.114 * b) / 255.0;
        }
    }
    out
}

#[derive(Debug, Clone)]
pub struct FrameStack {
    max_frames: usize,
    frame_len: usize,
    frames: VecDeque<Vec<f32>>,
}

impl FrameStack {
    pub fn new(max_frames: usize, frame_len: usize) -> Self {
        Self { max_frames, frame_len, frames: VecDeque::new() }
    }

    pub fn push(&mut self, frame: Vec<f32>) {
        assert_eq!(frame.len(), self.frame_len);
        if self.frames.len() == self.max_frames {
            self.frames.pop_front();
        }
        self.frames.push_back(frame);
    }

    pub fn as_slices(&self) -> Vec<&[f32]> {
        self.frames.iter().map(Vec::as_slice).collect()
    }
}
```

Create `crates/nes-ai/src/profiles/mod.rs`:

```rust
pub mod smb;
```

Create `crates/nes-ai/src/profiles/smb.rs`:

```rust
use nes_core::NesCore;

use crate::{config::AiProfileConfig, profile::TaskProfile};

#[derive(Debug, Clone, PartialEq)]
pub struct SmbFeatures {
    pub level_progress: f32,
    pub horizontal_speed: f32,
    pub vertical_speed: f32,
    pub airborne: bool,
    pub player_state: u8,
    pub lives: u8,
}

#[derive(Debug, Clone)]
pub struct SmbProfile {
    config: AiProfileConfig,
}

impl SmbProfile {
    pub fn new(config: AiProfileConfig) -> Self {
        Self { config }
    }
}

impl TaskProfile for SmbProfile {
    type Features = SmbFeatures;

    fn config(&self) -> &AiProfileConfig {
        &self.config
    }

    fn decode_features(&self, core: &NesCore) -> Self::Features {
        SmbFeatures {
            level_progress: f32::from(core.read_memory(0x006D)) * 256.0 + f32::from(core.read_memory(0x0086)),
            horizontal_speed: f32::from(core.read_memory(0x0057) as i8),
            vertical_speed: f32::from(core.read_memory(0x009F) as i8),
            airborne: core.read_memory(0x001D) != 0,
            player_state: core.read_memory(0x000E),
            lives: core.read_memory(0x075A),
        }
    }
}
```

Update `crates/nes-ai/src/lib.rs`:

```rust
pub mod actions;
pub mod config;
pub mod error;
pub mod obs;
pub mod profile;
pub mod profiles;
pub mod snapshot;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p nes-ai profile_config_parses_expected_control_defaults -- --exact`

Expected: PASS

Run: `cargo test -p nes-ai grayscale_downsample_outputs_expected_shape_and_unit_range -- --exact`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/nes-ai/src/lib.rs crates/nes-ai/src/config.rs crates/nes-ai/src/profile.rs crates/nes-ai/src/obs.rs crates/nes-ai/src/profiles/mod.rs crates/nes-ai/src/profiles/smb.rs crates/nes-ai/tests/profile_config.rs crates/nes-ai/tests/observation_contract.rs
git commit -m "feat(nes-ai): add profile config and observation preprocessing"
```

### Task 4: Add Reward Logic And Episode Artifact Writing

**Files:**
- Modify: `crates/nes-ai/src/lib.rs`
- Create: `crates/nes-ai/src/reward.rs`
- Create: `crates/nes-ai/src/episode.rs`
- Test: `crates/nes-ai/tests/reward_contract.rs`
- Test: `crates/nes-ai/tests/episode_artifacts.rs`

**Step 1: Write the failing tests**

Create `crates/nes-ai/tests/reward_contract.rs`:

```rust
use nes_ai::reward::{RewardBreakdown, RewardModel};
use nes_ai::{config::RewardConfig, profiles::smb::SmbFeatures};

#[test]
fn forward_progress_beats_stall_and_death_is_terminal() {
    let reward = RewardConfig {
        forward_progress: 1.0,
        alive_bonus: 0.01,
        stall_penalty: -0.02,
        death_penalty: -1.0,
        stall_frames: 120,
    };

    let model = RewardModel::new(reward);
    let prev = SmbFeatures {
        level_progress: 10.0,
        horizontal_speed: 0.0,
        vertical_speed: 0.0,
        airborne: false,
        player_state: 0x08,
        lives: 3,
    };
    let next = SmbFeatures { level_progress: 20.0, ..prev.clone() };

    let RewardBreakdown { total, done, .. } = model.score(&prev, &next, 0);
    assert!(total > 0.0);
    assert!(!done);

    let dead = SmbFeatures { player_state: 0x0B, ..next };
    let RewardBreakdown { total, done, .. } = model.score(&prev, &dead, 0);
    assert!(done);
    assert!(total < 0.0);
}
```

Create `crates/nes-ai/tests/episode_artifacts.rs`:

```rust
use nes_ai::episode::{EpisodeArtifactWriter, EpisodeMetadata};
use nes_core::tas::{TasFrameRun, TasMovie};
use tempfile::tempdir;

#[test]
fn episode_writer_emits_metadata_and_tas_json() {
    let dir = tempdir().unwrap();
    let writer = EpisodeArtifactWriter::new(dir.path().to_path_buf());
    let movie = TasMovie::from_runs(vec![TasFrameRun::new(0, 0, 2)]);
    let meta = EpisodeMetadata {
        profile_id: "smb-control".to_owned(),
        snapshot_id: "smb-control-v1".to_owned(),
        rom_hash: "rom-hash".to_owned(),
        total_reward: 1.5,
        episode_frames: 2,
        final_state_hash: 42,
    };

    let paths = writer.write("eval", &movie, &meta).unwrap();
    assert!(paths.tas_json_path.exists());
    assert!(paths.run_json_path.exists());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p nes-ai forward_progress_beats_stall_and_death_is_terminal -- --exact`

Expected: FAIL because `reward` does not exist.

Run: `cargo test -p nes-ai episode_writer_emits_metadata_and_tas_json -- --exact`

Expected: FAIL because `episode` does not exist.

**Step 3: Write minimal implementation**

Create `crates/nes-ai/src/reward.rs`:

```rust
use crate::{config::RewardConfig, profiles::smb::SmbFeatures};

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
    pub fn new(cfg: RewardConfig) -> Self {
        Self { cfg }
    }

    pub fn score(
        &self,
        prev: &SmbFeatures,
        next: &SmbFeatures,
        stalled_frames: u32,
    ) -> RewardBreakdown {
        let progress_delta = (next.level_progress - prev.level_progress) * self.cfg.forward_progress;
        let alive_bonus = self.cfg.alive_bonus;
        let stall_penalty = if stalled_frames >= self.cfg.stall_frames {
            self.cfg.stall_penalty
        } else {
            0.0
        };
        let dead = matches!(next.player_state, 0x06 | 0x0B) || next.lives < prev.lives;
        let death_penalty = if dead { self.cfg.death_penalty } else { 0.0 };
        RewardBreakdown {
            total: progress_delta + alive_bonus + stall_penalty + death_penalty,
            progress_delta,
            alive_bonus,
            stall_penalty,
            death_penalty,
            done: dead,
        }
    }
}
```

Create `crates/nes-ai/src/episode.rs`:

```rust
use std::{fs, path::PathBuf};

use nes_core::tas::TasMovie;
use serde::Serialize;

use crate::error::AiError;

#[derive(Debug, Clone, Serialize)]
pub struct EpisodeMetadata {
    pub profile_id: String,
    pub snapshot_id: String,
    pub rom_hash: String,
    pub total_reward: f32,
    pub episode_frames: u64,
    pub final_state_hash: u64,
}

#[derive(Debug, Clone)]
pub struct EpisodeArtifactPaths {
    pub tas_json_path: PathBuf,
    pub run_json_path: PathBuf,
    pub macro_txt_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct EpisodeArtifactWriter {
    output_dir: PathBuf,
}

impl EpisodeArtifactWriter {
    pub fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    pub fn write(
        &self,
        prefix: &str,
        movie: &TasMovie,
        meta: &EpisodeMetadata,
    ) -> Result<EpisodeArtifactPaths, AiError> {
        fs::create_dir_all(&self.output_dir).map_err(|_| AiError::Unsupported("artifact dir"))?;

        let tas_json_path = self.output_dir.join(format!("{prefix}.tas.json"));
        let run_json_path = self.output_dir.join(format!("{prefix}.run.json"));
        fs::write(&tas_json_path, serde_json::to_vec_pretty(movie).unwrap())
            .map_err(|_| AiError::Unsupported("artifact tas write"))?;
        fs::write(&run_json_path, serde_json::to_vec_pretty(meta).unwrap())
            .map_err(|_| AiError::Unsupported("artifact run write"))?;

        let macro_txt_path = movie.to_macro_script().ok().map(|script| {
            let path = self.output_dir.join(format!("{prefix}.macro.txt"));
            fs::write(&path, script).expect("macro artifact write");
            path
        });

        Ok(EpisodeArtifactPaths { tas_json_path, run_json_path, macro_txt_path })
    }
}
```

Update `crates/nes-ai/src/lib.rs`:

```rust
pub mod actions;
pub mod config;
pub mod episode;
pub mod error;
pub mod obs;
pub mod profile;
pub mod profiles;
pub mod reward;
pub mod snapshot;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p nes-ai forward_progress_beats_stall_and_death_is_terminal -- --exact`

Expected: PASS

Run: `cargo test -p nes-ai episode_writer_emits_metadata_and_tas_json -- --exact`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/nes-ai/src/lib.rs crates/nes-ai/src/reward.rs crates/nes-ai/src/episode.rs crates/nes-ai/tests/reward_contract.rs crates/nes-ai/tests/episode_artifacts.rs
git commit -m "feat(nes-ai): add reward model and episode artifact writing"
```

### Task 5: Implement The Profile-Driven Environment And Determinism Tests

**Files:**
- Modify: `crates/nes-ai/src/lib.rs`
- Create: `crates/nes-ai/src/env.rs`
- Create: `crates/nes-ai/tests/support/mod.rs`
- Create: `crates/nes-ai/tests/support/mock_profile.rs`
- Test: `crates/nes-ai/tests/env_determinism.rs`
- Test: `crates/nes-ai/tests/smb_control_integration.rs`

**Step 1: Write the failing tests**

Create `crates/nes-ai/tests/env_determinism.rs`:

```rust
mod support;

use nes_ai::actions::ControlAction;
use support::mock_profile::make_mock_env;

#[test]
fn reset_and_replayed_action_sequence_are_deterministic() {
    let mut a = make_mock_env();
    let mut b = make_mock_env();

    let _ = a.reset().unwrap();
    let _ = b.reset().unwrap();

    for action in [
        ControlAction::Right,
        ControlAction::RightA,
        ControlAction::Noop,
        ControlAction::RightB,
    ] {
        a.step(action).unwrap();
        b.step(action).unwrap();
    }

    assert_eq!(a.core().state_hash(), b.core().state_hash());
}
```

Create `crates/nes-ai/tests/smb_control_integration.rs`:

```rust
use std::path::PathBuf;

use nes_ai::{actions::ControlAction, config::AiProfileConfig, env::SmbControlEnv};

#[test]
#[ignore = "requires local SMB ROM and generated control snapshot"]
fn smb_control_profile_can_reset_and_gain_forward_reward() {
    let cfg: AiProfileConfig = toml::from_str(
        &std::fs::read_to_string(PathBuf::from("config/ai/profiles/smb-control.toml")).unwrap(),
    )
    .unwrap();

    let mut env = SmbControlEnv::from_config(cfg).unwrap();
    let _ = env.reset().unwrap();
    let step = env.step(ControlAction::Right).unwrap();

    assert!(step.reward.total.is_finite());
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p nes-ai reset_and_replayed_action_sequence_are_deterministic -- --exact`

Expected: FAIL because `env` does not exist.

**Step 3: Write minimal implementation**

Create `crates/nes-ai/src/env.rs`:

```rust
use nes_core::{Command, NesCore, tas::TasRecorder};

use crate::{
    actions::ControlAction,
    config::AiProfileConfig,
    episode::EpisodeMetadata,
    error::AiError,
    obs::{FrameStack, downsample_grayscale},
    profile::TaskProfile,
    profiles::smb::{SmbFeatures, SmbProfile},
    reward::{RewardBreakdown, RewardModel},
    snapshot::{SnapshotBundle, load_snapshot_bundle},
};

#[derive(Debug, Clone)]
pub struct StepOutput<F> {
    pub features: F,
    pub reward: RewardBreakdown,
    pub done: bool,
}

pub struct ProfileEnv<P: TaskProfile> {
    core: NesCore,
    profile: P,
    snapshot: SnapshotBundle,
    frame_stack: FrameStack,
    reward: RewardModel,
    recorder: TasRecorder,
    stalled_frames: u32,
    last_features: Option<SmbFeatures>,
}

impl<P: TaskProfile<Features = SmbFeatures>> ProfileEnv<P> {
    pub fn new(profile: P, snapshot: SnapshotBundle) -> Self {
        let cfg = profile.config().clone();
        Self {
            core: NesCore::new(),
            profile,
            snapshot,
            frame_stack: FrameStack::new(cfg.frame_stack, cfg.observation.width * cfg.observation.height),
            reward: RewardModel::new(cfg.reward),
            recorder: TasRecorder::new(),
            stalled_frames: 0,
            last_features: None,
        }
    }

    pub fn reset(&mut self) -> Result<SmbFeatures, AiError> {
        self.core.load_state(&self.snapshot.snapshot);
        self.recorder = TasRecorder::new();
        self.recorder.start();
        self.stalled_frames = 0;

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

    pub fn step(&mut self, action: ControlAction) -> Result<StepOutput<SmbFeatures>, AiError> {
        self.core.execute(Command::SetControllerState(action.controller1_bits()))
            .map_err(|_| AiError::Unsupported("step controller"))?;
        self.recorder.record_frame(action.controller1_bits());
        for _ in 0..self.profile.config().frame_skip {
            self.core.execute(Command::StepFrame).map_err(|_| AiError::Unsupported("step frame"))?;
        }

        let next = self.profile.decode_features(&self.core);
        let prev = self.last_features.clone().ok_or(AiError::Unsupported("step before reset"))?;
        self.stalled_frames = if next.level_progress > prev.level_progress {
            0
        } else {
            self.stalled_frames.saturating_add(self.profile.config().frame_skip)
        };
        let reward = self.reward.score(&prev, &next, self.stalled_frames);
        self.last_features = Some(next.clone());

        Ok(StepOutput { done: reward.done, reward, features: next })
    }

    pub fn core(&self) -> &NesCore {
        &self.core
    }

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
```

Create `crates/nes-ai/tests/support/mock_profile.rs`:

```rust
use std::path::PathBuf;

use nes_ai::{
    config::{AiProfileConfig, ObservationConfig, RewardConfig},
    env::ProfileEnv,
    profiles::smb::SmbProfile,
    snapshot::{SNAPSHOT_BUNDLE_VERSION, SnapshotBundle},
};
use nes_core::NesCore;

pub fn make_mock_env() -> ProfileEnv<SmbProfile> {
    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);

    let cfg = AiProfileConfig {
        id: "mock-control".to_owned(),
        rom_path: PathBuf::from("mock.nes"),
        snapshot_path: PathBuf::from("mock.state.json"),
        bootstrap_tas_path: PathBuf::from("mock.tas.json"),
        frame_stack: 4,
        frame_skip: 1,
        max_episode_frames: 60,
        observation: ObservationConfig { width: 84, height: 84 },
        reward: RewardConfig {
            forward_progress: 1.0,
            alive_bonus: 0.01,
            stall_penalty: -0.02,
            death_penalty: -1.0,
            stall_frames: 30,
        },
    };

    let snapshot = SnapshotBundle {
        version: SNAPSHOT_BUNDLE_VERSION,
        rom_hash: "mock".to_owned(),
        snapshot_id: "mock-v1".to_owned(),
        snapshot: core.save_state(),
    };

    ProfileEnv::new(SmbProfile::new(cfg), snapshot)
}
```

Create `crates/nes-ai/tests/support/mod.rs`:

```rust
pub mod mock_profile;
```

Update `crates/nes-ai/src/lib.rs`:

```rust
pub mod actions;
pub mod config;
pub mod env;
pub mod episode;
pub mod error;
pub mod obs;
pub mod profile;
pub mod profiles;
pub mod reward;
pub mod snapshot;
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p nes-ai reset_and_replayed_action_sequence_are_deterministic -- --exact`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/nes-ai/src/lib.rs crates/nes-ai/src/env.rs crates/nes-ai/tests/support/mod.rs crates/nes-ai/tests/support/mock_profile.rs crates/nes-ai/tests/env_determinism.rs crates/nes-ai/tests/smb_control_integration.rs
git commit -m "feat(nes-ai): add deterministic profile-driven control environment"
```

### Task 6: Implement The Burn Hybrid Policy/Value Model

**Files:**
- Modify: `crates/nes-ai/src/lib.rs`
- Create: `crates/nes-ai/src/model.rs`
- Test: `crates/nes-ai/tests/model_contract.rs`

**Step 1: Write the failing test**

Create `crates/nes-ai/tests/model_contract.rs`:

```rust
use burn::backend::{Autodiff, NdArray};
use burn::tensor::Tensor;
use nes_ai::model::{HybridModelConfig, HybridPolicyValueNet};

type B = Autodiff<NdArray>;

#[test]
fn policy_model_emits_logits_and_value_for_batch() {
    let device = Default::default();
    let model = HybridPolicyValueNet::<B>::new(&device, &HybridModelConfig::new(4, 16, 6));

    let frames = Tensor::<B, 4>::zeros([2, 4, 84, 84], &device);
    let features = Tensor::<B, 2>::zeros([2, 16], &device);

    let out = model.forward(frames, features);
    assert_eq!(out.policy_logits.dims(), [2, 6]);
    assert_eq!(out.value.dims(), [2, 1]);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p nes-ai policy_model_emits_logits_and_value_for_batch -- --exact`

Expected: FAIL because `model` does not exist.

**Step 3: Write minimal implementation**

Create `crates/nes-ai/src/model.rs`:

```rust
use burn::{
    nn::{
        conv::{Conv2d, Conv2dConfig},
        pool::{AdaptiveAvgPool2d, AdaptiveAvgPool2dConfig},
        Linear, LinearConfig, Relu,
    },
    prelude::*,
};

#[derive(Debug, Clone)]
pub struct HybridModelConfig {
    pub frame_stack: usize,
    pub feature_count: usize,
    pub action_count: usize,
}

impl HybridModelConfig {
    pub const fn new(frame_stack: usize, feature_count: usize, action_count: usize) -> Self {
        Self { frame_stack, feature_count, action_count }
    }
}

#[derive(Module, Debug)]
pub struct HybridPolicyValueNet<B: Backend> {
    conv1: Conv2d<B>,
    conv2: Conv2d<B>,
    pool: AdaptiveAvgPool2d,
    activation: Relu,
    feature_proj: Linear<B>,
    trunk: Linear<B>,
    policy_head: Linear<B>,
    value_head: Linear<B>,
}

#[derive(Debug)]
pub struct HybridForwardOutput<B: Backend> {
    pub policy_logits: Tensor<B, 2>,
    pub value: Tensor<B, 2>,
}

impl<B: Backend> HybridPolicyValueNet<B> {
    pub fn new(device: &B::Device, cfg: &HybridModelConfig) -> Self {
        let conv1 = Conv2dConfig::new([cfg.frame_stack, 16], [8, 8]).with_stride([4, 4]).init(device);
        let conv2 = Conv2dConfig::new([16, 32], [4, 4]).with_stride([2, 2]).init(device);
        let pool = AdaptiveAvgPool2dConfig::new([1, 1]).init();
        let feature_proj = LinearConfig::new(cfg.feature_count, 32).init(device);
        let trunk = LinearConfig::new(64, 64).init(device);
        let policy_head = LinearConfig::new(64, cfg.action_count).init(device);
        let value_head = LinearConfig::new(64, 1).init(device);
        Self { conv1, conv2, pool, activation: Relu::new(), feature_proj, trunk, policy_head, value_head }
    }

    pub fn forward(&self, frames: Tensor<B, 4>, features: Tensor<B, 2>) -> HybridForwardOutput<B> {
        let vision = self.conv1.forward(frames);
        let vision = self.activation.forward(vision);
        let vision = self.conv2.forward(vision);
        let vision = self.activation.forward(vision);
        let vision = self.pool.forward(vision).flatten(1, 3);

        let features = self.activation.forward(self.feature_proj.forward(features));
        let joined = Tensor::cat(vec![vision, features], 1);
        let trunk = self.activation.forward(self.trunk.forward(joined));

        HybridForwardOutput {
            policy_logits: self.policy_head.forward(trunk.clone()),
            value: self.value_head.forward(trunk),
        }
    }
}
```

Update `crates/nes-ai/src/lib.rs`:

```rust
pub mod actions;
pub mod config;
pub mod env;
pub mod episode;
pub mod error;
pub mod model;
pub mod obs;
pub mod profile;
pub mod profiles;
pub mod reward;
pub mod snapshot;
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p nes-ai policy_model_emits_logits_and_value_for_batch -- --exact`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/nes-ai/src/lib.rs crates/nes-ai/src/model.rs crates/nes-ai/tests/model_contract.rs
git commit -m "feat(nes-ai): add burn hybrid policy and value model"
```

### Task 7: Implement PPO Training, Evaluation, And Smoke Tests

**Files:**
- Modify: `crates/nes-ai/src/lib.rs`
- Create: `crates/nes-ai/src/trainer.rs`
- Create: `crates/nes-ai/src/bin/train_smb_control.rs`
- Create: `crates/nes-ai/src/bin/eval_smb_control.rs`
- Test: `crates/nes-ai/tests/trainer_smoke.rs`

**Step 1: Write the failing test**

Create `crates/nes-ai/tests/trainer_smoke.rs`:

```rust
use nes_ai::trainer::{TrainerConfig, evaluate_random_policy, run_mock_ppo_smoke};

#[test]
fn ppo_smoke_improves_mock_env_return_over_random_baseline() {
    let cfg = TrainerConfig::smoke();
    let baseline = evaluate_random_policy(&cfg, 8).unwrap();
    let trained = run_mock_ppo_smoke(&cfg, 8).unwrap();

    assert!(trained.average_return >= baseline.average_return);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p nes-ai ppo_smoke_improves_mock_env_return_over_random_baseline -- --exact`

Expected: FAIL because `trainer` does not exist.

**Step 3: Write minimal implementation**

Create `crates/nes-ai/src/trainer.rs`:

```rust
use burn::backend::{Autodiff, NdArray};

use crate::{actions::ControlAction, error::AiError};

pub type TrainBackend = Autodiff<NdArray>;

#[derive(Debug, Clone)]
pub struct TrainerConfig {
    pub seed: u64,
    pub rollout_steps: usize,
    pub minibatch_size: usize,
    pub epochs: usize,
    pub learning_rate: f64,
}

impl TrainerConfig {
    pub fn smoke() -> Self {
        Self {
            seed: 7,
            rollout_steps: 32,
            minibatch_size: 16,
            epochs: 2,
            learning_rate: 1e-3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EvalSummary {
    pub average_return: f32,
}

pub fn evaluate_random_policy(_cfg: &TrainerConfig, episodes: usize) -> Result<EvalSummary, AiError> {
    let _ = episodes;
    Ok(EvalSummary { average_return: 0.0 })
}

pub fn run_mock_ppo_smoke(_cfg: &TrainerConfig, episodes: usize) -> Result<EvalSummary, AiError> {
    let _ = episodes;
    Ok(EvalSummary { average_return: 0.5 })
}

pub fn action_from_arg(value: &str) -> Result<ControlAction, AiError> {
    match value {
        "noop" => Ok(ControlAction::Noop),
        "right" => Ok(ControlAction::Right),
        "right-a" => Ok(ControlAction::RightA),
        "a" => Ok(ControlAction::A),
        "right-b" => Ok(ControlAction::RightB),
        "right-a-b" => Ok(ControlAction::RightAB),
        _ => Err(AiError::Unsupported("unknown action")),
    }
}
```

Create `crates/nes-ai/src/bin/train_smb_control.rs`:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = nes_ai::trainer::TrainerConfig::smoke();
    let summary = nes_ai::trainer::run_mock_ppo_smoke(&cfg, 8)?;
    println!("average_return={}", summary.average_return);
    Ok(())
}
```

Create `crates/nes-ai/src/bin/eval_smb_control.rs`:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = nes_ai::trainer::TrainerConfig::smoke();
    let summary = nes_ai::trainer::evaluate_random_policy(&cfg, 8)?;
    println!("average_return={}", summary.average_return);
    Ok(())
}
```

Update `crates/nes-ai/src/lib.rs`:

```rust
pub mod actions;
pub mod config;
pub mod env;
pub mod episode;
pub mod error;
pub mod model;
pub mod obs;
pub mod profile;
pub mod profiles;
pub mod reward;
pub mod snapshot;
pub mod trainer;
```

Replace the stubbed trainer internals incrementally during execution with:

- rollout buffer
- GAE or discounted advantage computation
- PPO minibatch update
- checkpoint writing
- evaluation artifact emission

Do not jump straight to that full implementation before the smoke test exists and fails.

**Step 4: Run test to verify it passes**

Run: `cargo test -p nes-ai ppo_smoke_improves_mock_env_return_over_random_baseline -- --exact`

Expected: PASS

**Step 5: Commit**

```bash
git add crates/nes-ai/src/lib.rs crates/nes-ai/src/trainer.rs crates/nes-ai/src/bin/train_smb_control.rs crates/nes-ai/src/bin/eval_smb_control.rs crates/nes-ai/tests/trainer_smoke.rs
git commit -m "feat(nes-ai): add trainer skeleton and smoke coverage"
```

### Task 8: Wire Real PPO Internals, Document Usage, And Verify End-To-End

**Files:**
- Modify: `crates/nes-ai/src/env.rs`
- Modify: `crates/nes-ai/src/episode.rs`
- Modify: `crates/nes-ai/src/model.rs`
- Modify: `crates/nes-ai/src/trainer.rs`
- Modify: `crates/nes-ai/src/bin/train_smb_control.rs`
- Modify: `crates/nes-ai/src/bin/eval_smb_control.rs`
- Create: `crates/nes-ai/README.md`
- Modify: `README.md`
- Test: `crates/nes-ai/tests/trainer_smoke.rs`
- Test: `crates/nes-ai/tests/smb_control_integration.rs`

**Step 1: Replace the trainer stubs with real PPO**

Implement:

- seeded rollout collection
- discounted returns and GAE
- clipped PPO objective
- value loss
- entropy bonus
- checkpoint saving
- evaluation artifact emission through `EpisodeArtifactWriter`

Use the existing action space and `ProfileEnv` reset/step surface. Keep the smoke test fast by keeping the mock environment path and adding a separate ignored SMB evaluation path.

**Step 2: Update binaries and docs**

Create `crates/nes-ai/README.md` documenting:

- how to generate the fixed SMB control snapshot locally
- how to run the training binary
- how evaluation artifacts are laid out
- how TAS and macro exports can be replayed

Update the root `README.md` workspace section to mention `nes-ai`.

**Step 3: Run format**

Run: `cargo fmt --all`

Expected: PASS with no remaining diffs.

**Step 4: Run targeted tests**

Run: `cargo test -p nes-ai`

Expected: PASS for all ROM-free tests.

**Step 5: Run ignored SMB integration only when local assets exist**

Run: `cargo test -p nes-ai smb_control_profile_can_reset_and_gain_forward_reward -- --ignored --exact`

Expected: PASS when `config/ai/profiles/smb-control.toml`, the local SMB ROM, and the generated snapshot artifact all exist.

**Step 6: Run workspace regression**

Run: `cargo test --workspace`

Expected: PASS with `nes-ai` integrated into the workspace and no regressions elsewhere.

**Step 7: Review diff**

Run: `git diff -- crates/nes-ai README.md Cargo.toml config/ai/profiles/smb-control.example.toml`

Expected: One coherent `nes-ai` feature slice with tests, docs, snapshot prep, environment, model, and trainer.

**Step 8: Commit**

```bash
git add Cargo.toml README.md crates/nes-ai config/ai/profiles/smb-control.example.toml
git commit -m "feat(nes-ai): add burn-based smb control training stack"
```
