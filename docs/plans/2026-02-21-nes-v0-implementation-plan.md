# NES v0 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a proof-forward, MCP-first NES emulator workspace that reaches v0 gates: playable desktop/web and credible ROM test coverage.

**Architecture:** Keep `nes-core` deterministic and platform-agnostic, with all user-facing interactions flowing through command/query APIs mirrored by `nes-mcp` tools. Frontends (`nes-desktop`, `nes-web`) are thin adapters. `nes-proof` defines and proves critical invariants before or alongside runtime implementation.

**Tech Stack:** Rust 2024 workspace, Verus (`C:\Users\markm\verus\verus.exe`), `winit`, `pixels`, `cpal`, `wasm-bindgen`, `web-sys`, `proptest`, GitHub Actions.

---

## Preconditions

- Execute in branch: `feat/nes-v0-implementation-plan` worktree.
- Follow @test-driven-development for every behavior change.
- Use SPEC-PROOF-RED-GREEN-REFACTOR:
  1. spec/proof first for critical invariants
  2. failing runtime test
  3. minimal implementation
  4. refactor with proofs/tests green
- Run @verification-before-completion checks before each completion claim.

### Task 1: Bootstrap Workspace and Crate Topology

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/nes-core/Cargo.toml`
- Create: `crates/nes-core/src/lib.rs`
- Create: `crates/nes-mcp/Cargo.toml`
- Create: `crates/nes-mcp/src/lib.rs`
- Create: `crates/nes-mcp/src/main.rs`
- Create: `crates/nes-desktop/Cargo.toml`
- Create: `crates/nes-desktop/src/main.rs`
- Create: `crates/nes-web/Cargo.toml`
- Create: `crates/nes-web/src/lib.rs`
- Create: `crates/nes-proof/Cargo.toml`
- Create: `crates/nes-proof/src/lib.rs`
- Create: `crates/nes-test-harness/Cargo.toml`
- Create: `crates/nes-test-harness/src/lib.rs`

**Step 1: Write failing workspace check**

Edit root `Cargo.toml` to declare all workspace members before the member manifests exist.

**Step 2: Run check to verify failure**

Run: `cargo check --workspace`
Expected: FAIL with missing member manifest errors.

**Step 3: Add minimal crate manifests and stubs**

Use minimal compile stubs for each crate:

```rust
pub fn crate_bootstrap_marker() -> &'static str {
    "ok"
}
```

**Step 4: Run check to verify pass**

Run: `cargo check --workspace`
Expected: PASS for all workspace members.

**Step 5: Commit**

```bash
git add Cargo.toml crates/
git commit -m "chore: bootstrap NES workspace crates"
```

### Task 2: Define Core Command/Query Contract

**Files:**
- Create: `crates/nes-core/src/api.rs`
- Modify: `crates/nes-core/src/lib.rs`
- Test: `crates/nes-core/tests/command_query_contract.rs`

**Step 1: Write failing test**

```rust
use nes_core::{Command, CoreQuery, NesCore, QueryResult};

#[test]
fn boot_state_is_queryable_without_frontend() {
    let core = NesCore::new();
    let result = core.query(CoreQuery::EmulatorState);
    assert!(matches!(result, QueryResult::EmulatorState(_)));
}

#[test]
fn pause_and_resume_are_core_commands() {
    let mut core = NesCore::new();
    core.execute(Command::Pause).unwrap();
    assert!(core.is_paused());
    core.execute(Command::Resume).unwrap();
    assert!(!core.is_paused());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p nes-core command_query_contract -- --nocapture`
Expected: FAIL with missing types/methods.

**Step 3: Write minimal implementation**

Define:
- `enum Command`
- `enum CoreQuery`
- `enum QueryResult`
- `struct NesCore` with `new`, `execute`, `query`, `is_paused`

**Step 4: Run test to verify it passes**

Run: `cargo test -p nes-core command_query_contract -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-core
git commit -m "feat(core): add command/query contract skeleton"
```

### Task 3: Add Deterministic Cycle Scheduler

**Files:**
- Create: `crates/nes-core/src/scheduler.rs`
- Modify: `crates/nes-core/src/lib.rs`
- Test: `crates/nes-core/tests/scheduler_determinism.rs`

**Step 1: Write failing tests**

```rust
use nes_core::{Command, NesCore};

#[test]
fn step_frame_advances_monotonic_cycle_counter() {
    let mut core = NesCore::new();
    let c0 = core.total_cycles();
    core.execute(Command::StepFrame).unwrap();
    let c1 = core.total_cycles();
    assert!(c1 > c0);
}

#[test]
fn identical_command_sequences_yield_identical_state_hash() {
    let mut a = NesCore::new();
    let mut b = NesCore::new();
    for _ in 0..10 {
        a.execute(Command::StepCpu).unwrap();
        b.execute(Command::StepCpu).unwrap();
    }
    assert_eq!(a.state_hash(), b.state_hash());
}
```

**Step 2: Run tests to verify failure**

Run: `cargo test -p nes-core scheduler_determinism -- --nocapture`
Expected: FAIL with missing scheduler API.

**Step 3: Implement minimal deterministic scheduler**

Add:
- monotonic cycle counters
- `StepCpu` and `StepFrame` command handling
- stable state hash function over deterministic state fields

**Step 4: Run tests to verify pass**

Run: `cargo test -p nes-core scheduler_determinism -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-core
git commit -m "feat(core): add deterministic cycle scheduler skeleton"
```

### Task 4: Establish MCP Tool Catalog (All User-Facing I/O)

**Files:**
- Create: `crates/nes-mcp/src/tools.rs`
- Modify: `crates/nes-mcp/src/lib.rs`
- Test: `crates/nes-mcp/tests/tool_catalog.rs`

**Step 1: Write failing test**

```rust
use nes_mcp::tool_catalog;

#[test]
fn catalog_contains_required_user_facing_tools() {
    let tools = tool_catalog();
    for name in [
        "load_rom", "set_controller_state", "press_button", "release_button",
        "reset", "power_cycle", "pause", "resume", "set_speed",
        "get_frame", "get_audio_chunk", "get_fps", "get_emulator_state",
        "read_memory", "read_registers", "disassemble_at",
        "step_cpu", "step_scanline", "step_frame",
        "set_breakpoint", "clear_breakpoint", "save_state", "load_state",
    ] {
        assert!(tools.iter().any(|t| t.name == name), "missing {name}");
    }
}
```

**Step 2: Run test to verify failure**

Run: `cargo test -p nes-mcp tool_catalog -- --nocapture`
Expected: FAIL with missing tool catalog.

**Step 3: Implement minimal tool registry**

Expose static registry entries with names and short schemas.

**Step 4: Run test to verify pass**

Run: `cargo test -p nes-mcp tool_catalog -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-mcp
git commit -m "feat(mcp): add complete user-facing tool catalog"
```

### Task 5: Enforce MCP/Core Parity for Control Tools

**Files:**
- Create: `crates/nes-mcp/src/dispatch.rs`
- Modify: `crates/nes-mcp/src/lib.rs`
- Test: `crates/nes-mcp/tests/control_parity.rs`

**Step 1: Write failing parity test**

```rust
use nes_core::{Command, NesCore};
use nes_mcp::dispatch_tool;

#[test]
fn pause_tool_matches_direct_core_command() {
    let mut via_core = NesCore::new();
    via_core.execute(Command::Pause).unwrap();

    let mut via_mcp = NesCore::new();
    dispatch_tool(&mut via_mcp, "pause", serde_json::json!({})).unwrap();

    assert_eq!(via_core.state_hash(), via_mcp.state_hash());
}
```

**Step 2: Run test to verify failure**

Run: `cargo test -p nes-mcp control_parity -- --nocapture`
Expected: FAIL with missing dispatcher or parity mismatch.

**Step 3: Implement minimal dispatcher mapping**

Map control tools directly to `nes-core::Command` paths.

**Step 4: Run test to verify pass**

Run: `cargo test -p nes-mcp control_parity -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-mcp crates/nes-core
git commit -m "test(mcp): add control tool parity against core commands"
```

### Task 6: Bootstrap Verus Proof Crate and Verification Script

**Files:**
- Create: `crates/nes-proof/src/cpu_model.rs`
- Create: `scripts/verus-check.ps1`
- Modify: `crates/nes-proof/src/lib.rs`
- Test: `crates/nes-proof/src/cpu_model.rs` (proof assertions)

**Step 1: Write intentionally failing proof**

In `cpu_model.rs`, add a lemma with an unprovable assertion.

**Step 2: Run proof to verify failure**

Run: `C:\Users\markm\verus\verus.exe crates/nes-proof/src/cpu_model.rs`
Expected: FAIL with proof obligation unsatisfied.

**Step 3: Implement minimal proven model**

Replace failing assertion with:
- `CpuModel` spec fields
- basic invariants for 8-bit registers and flag bit legality
- at least one proved invariant-preservation lemma

**Step 4: Run proof to verify pass**

Run: `C:\Users\markm\verus\verus.exe crates/nes-proof/src/cpu_model.rs`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-proof scripts/verus-check.ps1
git commit -m "feat(proof): bootstrap verus proof checks and cpu model base"
```

### Task 7: Implement and Prove CPU Status Register Contracts

**Files:**
- Create: `crates/nes-core/src/cpu/status.rs`
- Create: `crates/nes-proof/src/status_flags.rs`
- Modify: `crates/nes-core/src/lib.rs`
- Test: `crates/nes-core/tests/status_flags.rs`

**Step 1: Write failing runtime test**

```rust
use nes_core::cpu::status::Status;

#[test]
fn zero_and_negative_flags_follow_value_written() {
    let mut s = Status::default();
    s.update_zn(0x00);
    assert!(s.zero());
    assert!(!s.negative());
    s.update_zn(0x80);
    assert!(!s.zero());
    assert!(s.negative());
}
```

**Step 2: Run runtime and proof checks to verify failure**

Run: `cargo test -p nes-core status_flags -- --nocapture`
Expected: FAIL with missing status behavior.

Run: `C:\Users\markm\verus\verus.exe crates/nes-proof/src/status_flags.rs`
Expected: FAIL with missing invariant proof.

**Step 3: Implement minimal runtime + proof**

Add status flag operations in runtime and matching Verus lemmas for legality/preservation.

**Step 4: Re-run checks to verify pass**

Run: `cargo test -p nes-core status_flags -- --nocapture`
Expected: PASS.

Run: `C:\Users\markm\verus\verus.exe crates/nes-proof/src/status_flags.rs`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-core crates/nes-proof
git commit -m "feat(cpu): prove and implement status flag contracts"
```

### Task 8: Prove and Implement Bus Region Mapping

**Files:**
- Create: `crates/nes-core/src/bus.rs`
- Create: `crates/nes-proof/src/bus_map.rs`
- Test: `crates/nes-core/tests/bus_map.rs`
- Test: `crates/nes-core/tests/bus_map_prop.rs`

**Step 1: Write failing tests**

```rust
use nes_core::bus::{BusRegion, map_region};

#[test]
fn address_regions_are_unambiguous() {
    let region = map_region(0x8000);
    assert_eq!(region, BusRegion::CartridgePrgRom);
}
```

Property test:

```rust
proptest! {
    #[test]
    fn every_address_maps_to_exactly_one_region(addr in any::<u16>()) {
        let region = map_region(addr);
        prop_assert!(region.is_legal());
    }
}
```

**Step 2: Run tests/proof to verify failure**

Run: `cargo test -p nes-core bus_map -- --nocapture`
Expected: FAIL with missing map behavior.

Run: `C:\Users\markm\verus\verus.exe crates/nes-proof/src/bus_map.rs`
Expected: FAIL with unsatisfied mapping obligations.

**Step 3: Implement minimal mapping and proof**

Define exhaustive address mapping and prove totality + non-overlap.

**Step 4: Re-run checks to verify pass**

Run: `cargo test -p nes-core bus_map -- --nocapture`
Expected: PASS.

Run: `C:\Users\markm\verus\verus.exe crates/nes-proof/src/bus_map.rs`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-core crates/nes-proof
git commit -m "feat(bus): prove and implement total non-overlapping bus map"
```

### Task 9: Implement + Prove NROM and UxROM Mapper Contracts

**Files:**
- Create: `crates/nes-core/src/mapper/mod.rs`
- Create: `crates/nes-core/src/mapper/nrom.rs`
- Create: `crates/nes-core/src/mapper/uxrom.rs`
- Create: `crates/nes-proof/src/mapper_nrom_uxrom.rs`
- Test: `crates/nes-core/tests/mapper_nrom_uxrom.rs`

**Step 1: Write failing tests**

```rust
use nes_core::mapper::{Mapper, Nrom, Uxrom};

#[test]
fn nrom_ignores_bank_switch_writes() {
    let mut m = Nrom::new_32k();
    let before = m.read_prg(0x8000);
    m.write_prg(0x8000, 1);
    assert_eq!(before, m.read_prg(0x8000));
}

#[test]
fn uxrom_switches_lower_bank_only() {
    let mut m = Uxrom::new(8);
    m.write_prg(0x8000, 3);
    assert_eq!(m.selected_bank(), 3);
}
```

**Step 2: Run tests/proof to verify failure**

Run: `cargo test -p nes-core mapper_nrom_uxrom -- --nocapture`
Expected: FAIL.

Run: `C:\Users\markm\verus\verus.exe crates/nes-proof/src/mapper_nrom_uxrom.rs`
Expected: FAIL.

**Step 3: Implement minimal runtime + proof**

Guarantee bank index bounds and fixed upper bank semantics for UxROM.

**Step 4: Re-run checks to verify pass**

Run: `cargo test -p nes-core mapper_nrom_uxrom -- --nocapture`
Expected: PASS.

Run: `C:\Users\markm\verus\verus.exe crates/nes-proof/src/mapper_nrom_uxrom.rs`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-core crates/nes-proof
git commit -m "feat(mapper): prove and implement nrom and uxrom contracts"
```

### Task 10: Implement + Prove MMC1 Shift Register and Bank Legality

**Files:**
- Create: `crates/nes-core/src/mapper/mmc1.rs`
- Create: `crates/nes-proof/src/mapper_mmc1.rs`
- Test: `crates/nes-core/tests/mapper_mmc1.rs`

**Step 1: Write failing tests**

```rust
use nes_core::mapper::Mmc1;

#[test]
fn mmc1_resets_shift_register_on_bit7_write() {
    let mut m = Mmc1::new(16, 8);
    m.write_prg(0xE000, 0x80);
    assert!(m.shift_is_reset());
}
```

**Step 2: Run tests/proof to verify failure**

Run: `cargo test -p nes-core mapper_mmc1 -- --nocapture`
Expected: FAIL.

Run: `C:\Users\markm\verus\verus.exe crates/nes-proof/src/mapper_mmc1.rs`
Expected: FAIL.

**Step 3: Implement minimal runtime + proof**

Implement 5-bit serial latch behavior and prove selected bank legality.

**Step 4: Re-run checks to verify pass**

Run: `cargo test -p nes-core mapper_mmc1 -- --nocapture`
Expected: PASS.

Run: `C:\Users\markm\verus\verus.exe crates/nes-proof/src/mapper_mmc1.rs`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-core crates/nes-proof
git commit -m "feat(mapper): prove and implement mmc1 shift and bank safety"
```

### Task 11: Add Desktop Adapter with Command Bridge

**Files:**
- Create: `crates/nes-desktop/src/app.rs`
- Modify: `crates/nes-desktop/src/main.rs`
- Test: `crates/nes-desktop/tests/input_bridge.rs`

**Step 1: Write failing bridge test**

```rust
use nes_desktop::app::map_key_event_to_command;

#[test]
fn keyboard_press_maps_to_controller_command() {
    let cmd = map_key_event_to_command("KeyZ", true).unwrap();
    assert_eq!(cmd.tool_name(), "press_button");
}
```

**Step 2: Run test to verify failure**

Run: `cargo test -p nes-desktop input_bridge -- --nocapture`
Expected: FAIL with missing bridge function.

**Step 3: Implement minimal bridge**

Map key events to core commands/tool-equivalent operations and no direct state mutation.

**Step 4: Run test to verify pass**

Run: `cargo test -p nes-desktop input_bridge -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-desktop
git commit -m "feat(desktop): add input bridge through core command surface"
```

### Task 12: Add Web Adapter with Command Bridge

**Files:**
- Create: `crates/nes-web/src/bridge.rs`
- Modify: `crates/nes-web/src/lib.rs`
- Test: `crates/nes-web/tests/web_bridge.rs`

**Step 1: Write failing test**

```rust
use nes_web::bridge::map_dom_key_to_command;

#[test]
fn dom_key_maps_to_press_button_command() {
    let cmd = map_dom_key_to_command("KeyX", true).unwrap();
    assert_eq!(cmd.tool_name(), "press_button");
}
```

**Step 2: Run test to verify failure**

Run: `cargo test -p nes-web web_bridge -- --nocapture`
Expected: FAIL.

**Step 3: Implement minimal bridge**

Provide wasm-safe mapping layer into core command surface.

**Step 4: Run test to verify pass**

Run: `cargo test -p nes-web web_bridge -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-web
git commit -m "feat(web): add browser input bridge through core command surface"
```

### Task 13: Hybrid MCP Output Contract (Metadata + Chunk Fetch)

**Files:**
- Create: `crates/nes-mcp/src/output.rs`
- Test: `crates/nes-mcp/tests/output_contract.rs`

**Step 1: Write failing tests**

```rust
use nes_mcp::{audio_chunk, frame_chunk, latest_output_metadata};

#[test]
fn metadata_reports_incrementing_frame_sequence() {
    let m0 = latest_output_metadata();
    let _ = frame_chunk(m0.frame_seq + 1);
    let m1 = latest_output_metadata();
    assert!(m1.frame_seq >= m0.frame_seq);
}
```

**Step 2: Run test to verify failure**

Run: `cargo test -p nes-mcp output_contract -- --nocapture`
Expected: FAIL.

**Step 3: Implement minimal metadata + fetch API**

Return metadata stream records and on-demand chunk fetch responses.

**Step 4: Run test to verify pass**

Run: `cargo test -p nes-mcp output_contract -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-mcp
git commit -m "feat(mcp): add hybrid output contract metadata and chunk fetch"
```

### Task 14: Deterministic Replay + Save/Load State Parity

**Files:**
- Create: `crates/nes-core/src/replay.rs`
- Create: `crates/nes-test-harness/tests/replay_determinism.rs`
- Modify: `crates/nes-core/src/lib.rs`

**Step 1: Write failing replay tests**

```rust
use nes_core::{Command, NesCore};

#[test]
fn recorded_command_log_replays_to_identical_state_hash() {
    let mut a = NesCore::new();
    let log = vec![Command::StepCpu, Command::Pause, Command::Resume, Command::StepFrame];
    for cmd in &log { a.execute(*cmd).unwrap(); }
    let target = a.state_hash();

    let mut b = NesCore::new();
    b.replay(&log).unwrap();
    assert_eq!(target, b.state_hash());
}
```

**Step 2: Run test to verify failure**

Run: `cargo test -p nes-test-harness replay_determinism -- --nocapture`
Expected: FAIL.

**Step 3: Implement minimal replay and save/load state hooks**

Add command log replay path and deterministic save/load representation.

**Step 4: Run test to verify pass**

Run: `cargo test -p nes-test-harness replay_determinism -- --nocapture`
Expected: PASS.

**Step 5: Commit**

```bash
git add crates/nes-core crates/nes-test-harness
git commit -m "feat(core): add deterministic replay and state parity scaffolding"
```

### Task 15: ROM Harness and v0 CI Quality Gates

**Files:**
- Create: `crates/nes-test-harness/tests/rom_nestest.rs`
- Create: `crates/nes-test-harness/tests/rom_blargg_cpu.rs`
- Create: `.github/workflows/ci.yml`
- Create: `docs/adr/0001-proof-forward-mcp-first.md`
- Modify: `README.md`

**Step 1: Write failing harness and gate checks**

Create ignored tests that expect ROM env vars and fail when required metadata/contracts are absent.

```rust
#[test]
#[ignore = "requires NESTEST_ROM_PATH"]
fn nestest_trace_matches_expected_prefix() { /* ... */ }
```

**Step 2: Run local checks to verify failure/signal**

Run: `cargo test -p nes-test-harness -- --nocapture`
Expected: PASS for non-ROM tests, ROM tests ignored.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: FAIL until all lint issues are addressed.

**Step 3: Implement minimal CI and docs**

CI stages:
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-targets --all-features`
- `powershell ./scripts/verus-check.ps1`

Add ADR and README gate definitions:
- playable target requirement
- ROM credibility requirement
- MCP/core parity requirement
- proof gate requirement

**Step 4: Run checks to verify pass**

Run:
- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-targets --all-features`
- `powershell ./scripts/verus-check.ps1`

Expected: PASS.

**Step 5: Commit**

```bash
git add .github/workflows/ci.yml docs/adr README.md crates/nes-test-harness scripts
git commit -m "chore(ci): enforce v0 gates for tests, proofs, and parity"
```

## Completion Checklist (Must Be Explicitly Verified)

- [ ] All MCP-required user-facing tools exist in `nes-mcp` catalog.
- [ ] No frontend mutates emulator state outside `nes-core` command/query APIs.
- [ ] Proof files for CPU flags, bus map, and mappers pass Verus checks.
- [ ] `NROM`, `UxROM`, `MMC1` tests and proofs are green.
- [ ] Replay determinism and state-hash parity tests pass.
- [ ] Desktop and web adapters build and pass bridge tests.
- [ ] CI enforces fmt, clippy, tests, and Verus checks.
- [ ] ADR and README document architecture and v0 gates.

## Suggested Command Bundle Before Any “Done” Claim

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
powershell ./scripts/verus-check.ps1
```
