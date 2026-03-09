# TAS Foundation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Move the macro recorder out of `nes-core` experimental status by replacing the string-only recorder with stable TAS primitives, exposed behind a dedicated `tas` feature flag, that can record, serialize, replay, and export legacy macro scripts.

**Architecture:** Add a new stable `nes_core::tas` module behind a dedicated `tas` Cargo feature. That module owns the public movie format (`TasMovie` + coalesced per-frame runs), recording API (`TasRecorder`), and deterministic playback helpers against `NesCore`. Keep the existing macro workflow alive by generating legacy macro scripts from the structured movie instead of treating script text as the source of truth.

**Tech Stack:** Rust 2024, `nes-core`, existing `serde`, existing `Command`/`CoreSnapshot`/`NesCore` APIs, cargo test.

---

### Task 1: Lock Public TAS Behavior With Failing Tests

**Files:**
- Create: `crates/nes-core/tests/tas.rs`

**Step 1: Write the failing test**

Write public API tests that cover:
- recorder coalesces identical input frames into run-length encoded TAS runs
- TAS replay reproduces the same `state_hash` as direct command execution
- TAS movie exports the legacy macro script sequence for player 1
- macro export rejects player-2 input until the legacy script format grows that capability
- TAS movie serde round-trips without losing run structure

**Step 2: Run test to verify it fails**

Run: `cargo test -p nes-core --test tas --features tas`
Expected: FAIL because `nes_core::tas` and its public types do not exist yet.

**Step 3: Commit**

```bash
git add crates/nes-core/tests/tas.rs
git commit -m "test(core): add TAS foundation public API tests"
```

### Task 2: Implement Stable TAS Core Types

**Files:**
- Create: `crates/nes-core/src/tas.rs`
- Modify: `crates/nes-core/src/lib.rs`
- Modify: `crates/nes-core/src/experimental/mod.rs`

**Step 1: Write minimal implementation**

Implement:
- `TasFrameRun` for `(controller1_bits, controller2_bits, frames)`
- `TasMovie` with `runs()`, `total_frames()`, `replay(&mut NesCore)`, and legacy macro export
- `TasRecorder` with `start`, `stop`, `clear`, `record_frame`, `record_frame_bits`, `record_core_frame`, `movie`, and `finish`
- `TasError` for macro-export limitations
- stable `pub mod tas;` export from `nes-core` behind the `tas` feature
- remove `macro_recorder` from `experimental` so the recorder is no longer gated behind `nova`

**Step 2: Run tests to verify they pass**

Run: `cargo test -p nes-core --test tas --features tas`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/nes-core/src/tas.rs crates/nes-core/src/lib.rs crates/nes-core/src/experimental/mod.rs crates/nes-core/tests/tas.rs
git commit -m "feat(core): add stable TAS movie and recorder primitives"
```

### Task 3: Document The New Stable Surface

**Files:**
- Modify: `crates/nes-core/README.md`
- Modify: `README.md`

**Step 1: Update docs**

Document:
- stable `nes_core::tas` module purpose
- deterministic replay role in future tooling / automation
- macro-script export as compatibility bridge

**Step 2: Run focused tests again**

Run: `cargo test -p nes-core --test tas --features tas`
Expected: PASS

**Step 3: Commit**

```bash
git add crates/nes-core/README.md README.md
git commit -m "docs: document stable TAS foundation"
```

### Task 4: Final Verification

**Files:**
- Modify: `crates/nes-core/src/tas.rs`
- Modify: `crates/nes-core/tests/tas.rs`
- Modify: `crates/nes-core/README.md`
- Modify: `README.md`

**Step 1: Run cargo fmt**

Run: `cargo fmt --all`
Expected: PASS with no formatting diffs left behind.

**Step 2: Run targeted tests**

Run: `cargo test -p nes-core --test tas --features tas`
Expected: PASS

**Step 3: Run crate tests**

Run: `cargo test -p nes-core --features tas`
Expected: PASS

**Step 4: Verify default non-TAS crate path still works**

Run: `cargo test -p nes-core`
Expected: PASS

**Step 5: Review diff**

Run: `git diff -- crates/nes-core/src/tas.rs crates/nes-core/tests/tas.rs crates/nes-core/README.md README.md crates/nes-core/src/lib.rs crates/nes-core/src/experimental/mod.rs`
Expected: Diff shows one coherent TAS foundation change set with tests and docs.
