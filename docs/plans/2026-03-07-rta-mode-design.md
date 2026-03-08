# RTA Mode (Speedrunner-Focused) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an explicit desktop RTA mode with memory-triggered timing, profile auto-selection by ROM hash, strict invalidation rules, run artifact persistence, and calibration-to-draft profile generation.

**Architecture:** Implement an `rta` module inside `nes-desktop` that owns profile loading, trigger evaluation, run state machine, invalidation policy, split recording, and artifact writing. Wire it into `main.rs` as an optional runtime mode activated by CLI flags, leaving normal emulation behavior untouched. Keep profile storage external (`config/rta/profiles/*.toml`) and enforce draft-vs-published profile semantics.

**Tech Stack:** Rust 2024, `serde`/`toml`/`serde_json`, desktop runtime loop in `winit`, deterministic core memory reads via `nes_core::NesCore`.

---

### Task 1: Add RTA Domain Module Skeleton

**Files:**
- Create: `crates/nes-desktop/src/rta.rs`
- Modify: `crates/nes-desktop/src/main.rs`
- Test: `crates/nes-desktop/src/rta.rs`

**Step 1: Write failing tests**
- Add unit tests for:
  - profile status defaults (`published`)
  - trigger op parsing
  - unknown-field rejection in profile TOML

**Step 2: Run test to verify it fails**
- Run: `cargo test -p nes-desktop rta::tests::profile_parse_rejects_unknown_fields`
- Expected: FAIL due to missing module/types.

**Step 3: Write minimal implementation**
- Add profile and trigger data types with serde derives:
  - `RtaProfile`, `ProfileStatus`, `TimerPolicy`, `SplitPolicy`, `LoggingPolicy`, `TriggerRule`
- Add `mod rta;` in `main.rs`.

**Step 4: Run tests to verify pass**
- Run: `cargo test -p nes-desktop rta::tests::profile_parse_rejects_unknown_fields`

---

### Task 2: Implement Trigger Engine + State Machine

**Files:**
- Modify: `crates/nes-desktop/src/rta.rs`
- Test: `crates/nes-desktop/src/rta.rs`

**Step 1: Write failing tests**
- Add tests for:
  - `eq`, `changed`, `bit_set`, `bit_clear` trigger semantics
  - debounce/consecutive behavior
  - transitions: `Armed -> Running -> Finished`
  - invalidation keeps timer active in `InvalidPractice`

**Step 2: Run tests to verify failures**
- Run: `cargo test -p nes-desktop rta::tests::state_machine_transitions_and_invalidation`

**Step 3: Write minimal implementation**
- Add:
  - `RtaSessionState`
  - `TriggerEngine`
  - `RtaManager` with per-frame `tick(memory_reader, frame, now)`
  - invalidation API `mark_forbidden_action`.

**Step 4: Run tests**
- Run: `cargo test -p nes-desktop rta::tests::`

---

### Task 3: Profile Loader + ROM Hash Auto-Selection

**Files:**
- Modify: `crates/nes-desktop/src/rta.rs`
- Modify: `crates/nes-desktop/Cargo.toml`
- Test: `crates/nes-desktop/src/rta.rs`

**Step 1: Write failing tests**
- Add tests for:
  - loading profiles from directory
  - selecting by ROM hash
  - rejecting draft profile in strict mode
  - manual override choosing exact profile id

**Step 2: Run failing tests**
- Run: `cargo test -p nes-desktop rta::tests::select_profile`

**Step 3: Implement**
- Add stable ROM hash computation helper.
- Add profile directory scanning and selection logic:
  - auto-select by hash
  - optional manual override
  - strict mode: published only.

**Step 4: Run tests**
- Run: `cargo test -p nes-desktop rta::tests::select_profile`

---

### Task 4: Run Artifacts (`run.json` + Optional Input Log)

**Files:**
- Modify: `crates/nes-desktop/src/rta.rs`
- Test: `crates/nes-desktop/src/rta.rs`

**Step 1: Write failing tests**
- Add tests for:
  - always writes `run.json`
  - writes input log only when enabled
  - invalidation reason persisted

**Step 2: Run failing tests**
- Run: `cargo test -p nes-desktop rta::tests::artifact_writer`

**Step 3: Implement**
- Add serializable run artifact model and file writer.
- Add per-frame input-log buffering behind `save_input_log` toggle.

**Step 4: Run tests**
- Run: `cargo test -p nes-desktop rta::tests::artifact_writer`

---

### Task 5: Calibration Session -> Draft Profile Output

**Files:**
- Modify: `crates/nes-desktop/src/rta.rs`
- Test: `crates/nes-desktop/src/rta.rs`

**Step 1: Write failing tests**
- Add tests for:
  - calibration session records manual splits
  - memory trace capture emits candidate triggers
  - outputs `<id>.draft.toml` and report JSON

**Step 2: Run failing tests**
- Run: `cargo test -p nes-desktop rta::tests::calibration_outputs_draft_profile`

**Step 3: Implement**
- Add `CalibrationRecorder` and draft writer.
- Keep heuristic simple and deterministic (value change around split frames).

**Step 4: Run tests**
- Run: `cargo test -p nes-desktop rta::tests::calibration_outputs_draft_profile`

---

### Task 6: Wire RTA into Desktop Runtime + CLI

**Files:**
- Modify: `crates/nes-desktop/src/main.rs`
- Modify: `README.md`
- Modify: `nes.example.toml` (optional references only)
- Test: `crates/nes-desktop/src/main.rs`

**Step 1: Write failing tests**
- Add parse-runtime-args tests for:
  - `--rta`
  - `--rta-profile`
  - `--rta-profiles-dir`
  - `--rta-calibrate`
- Add behavior tests for:
  - no prompt outside RTA mode
  - strict block when RTA mode has no matching profile.

**Step 2: Run failing tests**
- Run: `cargo test -p nes-desktop parse_runtime_args_accepts_rta_flags`

**Step 3: Implement runtime wiring**
- Load profiles only when `--rta` enabled.
- Auto-select profile by ROM hash with pre-start override support.
- Block entering RTA mode if no profile selected/matched.
- Hook run loop:
  - call RTA tick each frame
  - manual split hotkey
  - invalidation on rewind/save-load/frame-step actions.

**Step 4: Run tests**
- Run: `cargo test -p nes-desktop parse_runtime_args_accepts_rta_flags`

---

### Task 7: End-to-End Verification

**Files:**
- Modify: any touched files

**Step 1: Format**
- Run: `cargo fmt --all`

**Step 2: Targeted tests**
- Run: `cargo test -p nes-desktop --lib`

**Step 3: Workspace confidence pass**
- Run: `cargo test -p nes-desktop`

**Step 4: Documentation verification**
- Ensure README includes:
  - RTA mode intent
  - profile file locations
  - strict/draft behavior
  - calibration draft flow.

**Step 5: Final self-review**
- Run: `git diff --stat` and check for TODO/FIXME/stubs in touched areas.

