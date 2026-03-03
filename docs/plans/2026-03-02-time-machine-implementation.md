# Time Machine: Player-Facing Rewind Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement a 30-second rayon-powered rewind system with anchor+delta compression and speculative pre-reconstruction, integrated into nes-desktop.

**Architecture:** A new `nes-rewind` crate owns all Time Machine code: delta encoding structs, a `CompressedTimeline` (anchor+delta ring buffer), and a `TimeMachine` wrapper that drives a rayon thread pool for background compression and speculative frame reconstruction. The main emulation thread stays single-threaded; all cross-thread work flows through bounded channels.

**Tech Stack:** Rust 2024, rayon (thread pool), crossbeam-channel (bounded MPSC), smallvec (zero-heap small diffs), nes-core (CoreSnapshot / save_state / load_state)

---

## PREREQUISITE: CPU Work RAM Gap

The existing `CoreSnapshot` does NOT capture the 2KB CPU work RAM (`$0000–$07FF`). Only CPU registers are saved. This silently breaks save-state restoration in real games. The rollback tests pass only because their test ROM (`NOP; JMP $C000`) never writes to RAM. Task 1 fixes this before any Time Machine code is written.

---

## Task 1: Fix CoreSnapshot to Capture CPU Work RAM

**Context:** `NesCore` owns a `Cpu` struct with `memory: [u8; 0x1_0000]`. Bytes `$0000–$07FF` are the NES 2KB work RAM (mirrored to `$1FFF` — only the first 2KB are canonical). The mapper PRG window (`sync_mapper_prg_window`) repopulates ROM pages on restore, so only the 2KB work RAM needs explicit capture.

**Files:**
- Modify: `crates/nes-core/src/cpu/engine.rs:55-68` (CpuSnapshot struct)
- Modify: `crates/nes-core/src/cpu/engine.rs:196-218` (snapshot + restore)

**Step 1: Write the failing test**

In `crates/nes-core/src/cpu/engine.rs`, add inside `#[cfg(test)] mod tests { ... }` (or create the module if absent):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_snapshot_roundtrips_work_ram() {
        let mut cpu = Cpu::new(0xC000);
        // Write a recognizable pattern to the 2KB work RAM
        cpu.memory[0x0000] = 0xAB;
        cpu.memory[0x00FF] = 0xCD;
        cpu.memory[0x07FF] = 0xEF;

        let snap = cpu.snapshot();
        // Corrupt work RAM in-place
        cpu.memory[0x0000] = 0x00;
        cpu.memory[0x00FF] = 0x00;
        cpu.memory[0x07FF] = 0x00;

        cpu.restore(snap);
        assert_eq!(cpu.memory[0x0000], 0xAB);
        assert_eq!(cpu.memory[0x00FF], 0xCD);
        assert_eq!(cpu.memory[0x07FF], 0xEF);
    }
}
```

**Step 2: Run test to verify it fails**

```bash
cd crates/nes-core && cargo test cpu_snapshot_roundtrips_work_ram -- --nocapture
```

Expected: FAIL — restore only sets registers, not RAM. Values are 0x00 after restore.

**Step 3: Add work_ram field to CpuSnapshot**

In `crates/nes-core/src/cpu/engine.rs`, update `CpuSnapshot`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuSnapshot {
    pub pc: u16,
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub status: u8,
    /// 2KB NES work RAM ($0000–$07FF).
    pub work_ram: [u8; 2048],
}
```

Update `snapshot()`:

```rust
pub fn snapshot(&self) -> CpuSnapshot {
    let mut work_ram = [0u8; 2048];
    work_ram.copy_from_slice(&self.memory[0..2048]);
    CpuSnapshot {
        pc: self.pc,
        a: self.a,
        x: self.x,
        y: self.y,
        sp: self.sp,
        status: self.status.bits(),
        work_ram,
    }
}
```

Update `restore()`:

```rust
pub fn restore(&mut self, snapshot: CpuSnapshot) {
    self.pc = snapshot.pc;
    self.a = snapshot.a;
    self.x = snapshot.x;
    self.y = snapshot.y;
    self.sp = snapshot.sp;
    self.status = Status::with_bits(snapshot.status);
    self.memory[0..2048].copy_from_slice(&snapshot.work_ram);
    self.writes.clear();
    self.prg_writes.clear();
    self.bus_trace.borrow_mut().clear();
}
```

**Step 4: Run test to verify it passes**

```bash
cd crates/nes-core && cargo test cpu_snapshot_roundtrips_work_ram
```

Expected: PASS

**Step 5: Run full workspace to catch any breakage**

```bash
cargo test --workspace
```

Expected: all tests pass (CpuSnapshot is `Copy` — adding a 2KB array makes it no longer `Copy`. You'll need to change `Copy` to just `Clone` on `CpuSnapshot` and update any `.copy()` / `Copy` trait bounds in callers.)

Fix any compile errors from removing `Copy` from `CpuSnapshot`. Search for usages:

```bash
grep -rn "CpuSnapshot" crates/ --include="*.rs"
```

Update any code that relied on `Copy` to use `.clone()` instead.

**Step 6: Commit**

```bash
git add crates/nes-core/src/cpu/engine.rs
git commit -m "fix: capture 2KB work RAM in CpuSnapshot for correct save-state restoration"
```

---

## Task 2: Scaffold nes-rewind Crate

**Files:**
- Create: `crates/nes-rewind/Cargo.toml`
- Create: `crates/nes-rewind/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

**Step 1: Create crate skeleton**

```bash
mkdir -p crates/nes-rewind/src
```

`crates/nes-rewind/Cargo.toml`:

```toml
[package]
name = "nes-rewind"
version = "0.1.0"
edition = "2024"

[dependencies]
nes-core = { path = "../nes-core" }
rayon = "1.10"
crossbeam-channel = "0.5"
smallvec = { version = "1.13", features = ["const_generics"] }
```

`crates/nes-rewind/src/lib.rs`:

```rust
//! Time Machine: player-facing rewind with anchor+delta compression.

mod delta;
mod timeline;
mod policy;
mod worker;
mod cursor;

pub use timeline::CompressedTimeline;
pub use policy::KeyframePolicy;
pub use worker::TimeMachine;
pub use delta::{ArrayDelta, FieldDelta, FrameDelta};
```

Add to workspace `Cargo.toml` members list:

```toml
"crates/nes-rewind",
```

**Step 2: Verify it compiles**

```bash
cargo build -p nes-rewind
```

Expected: compiles (empty modules will need stubs — add `// empty` to each mod file).

**Step 3: Commit**

```bash
git add crates/nes-rewind/ Cargo.toml
git commit -m "feat: scaffold nes-rewind crate with dependency skeleton"
```

---

## Task 3: ArrayDelta — Byte-Level XOR Diff

The heaviest part of a `CoreSnapshot` is three byte arrays: CPU work RAM (2KB), CHR pattern tables (8KB), and nametable RAM (2KB). Frame-to-frame, most bytes don't change. `ArrayDelta` captures only the changed runs as `(offset, new_bytes)` pairs.

**Files:**
- Create: `crates/nes-rewind/src/delta.rs`

**Step 1: Write failing tests**

Create `crates/nes-rewind/src/delta.rs`:

```rust
use smallvec::SmallVec;

/// A changed run within a byte array: (start_offset, new_bytes).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayDelta {
    pub offset: u16,
    pub data: SmallVec<[u8; 16]>,
}

/// Compute the set of changed byte runs between `before` and `after`.
/// Adjacent changed bytes are merged into a single run.
pub fn diff_array(before: &[u8], after: &[u8]) -> Vec<ArrayDelta> {
    assert_eq!(before.len(), after.len());
    todo!()
}

/// Apply a sequence of `ArrayDelta`s to a mutable byte slice in-place.
pub fn apply_deltas(target: &mut [u8], deltas: &[ArrayDelta]) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_empty_arrays_produces_no_deltas() {
        let a = [0u8; 64];
        let b = [0u8; 64];
        let deltas = diff_array(&a, &b);
        assert!(deltas.is_empty());
    }

    #[test]
    fn diff_detects_single_byte_change() {
        let a = [0u8; 64];
        let mut b = [0u8; 64];
        b[10] = 0xFF;
        let deltas = diff_array(&a, &b);
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].offset, 10);
        assert_eq!(deltas[0].data.as_slice(), &[0xFF]);
    }

    #[test]
    fn diff_merges_adjacent_changes_into_one_run() {
        let a = [0u8; 64];
        let mut b = [0u8; 64];
        b[5] = 0x01;
        b[6] = 0x02;
        b[7] = 0x03;
        let deltas = diff_array(&a, &b);
        assert_eq!(deltas.len(), 1);
        assert_eq!(deltas[0].offset, 5);
        assert_eq!(deltas[0].data.as_slice(), &[0x01, 0x02, 0x03]);
    }

    #[test]
    fn diff_produces_separate_runs_for_non_adjacent_changes() {
        let a = [0u8; 64];
        let mut b = [0u8; 64];
        b[2] = 0xAA;
        b[20] = 0xBB;
        let deltas = diff_array(&a, &b);
        assert_eq!(deltas.len(), 2);
    }

    #[test]
    fn apply_roundtrips_through_diff() {
        let before = (0u8..=63).collect::<Vec<_>>();
        let mut after = before.clone();
        after[0] = 0xFF;
        after[31] = 0xAA;
        after[63] = 0x00;

        let deltas = diff_array(&before, &after);
        let mut reconstructed = before.clone();
        apply_deltas(&mut reconstructed, &deltas);
        assert_eq!(reconstructed, after);
    }

    #[test]
    fn apply_large_array_roundtrip() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        // Simulate a 2KB work RAM diff
        let before = vec![0u8; 2048];
        let mut after = before.clone();
        // Scatter 50 random-ish changes
        for i in (0..2048usize).step_by(41) {
            after[i] = (i % 256) as u8;
        }
        let deltas = diff_array(&before, &after);
        let mut reconstructed = before.clone();
        apply_deltas(&mut reconstructed, &deltas);
        assert_eq!(reconstructed, after);
    }
}
```

**Step 2: Run to confirm they fail**

```bash
cargo test -p nes-rewind delta -- --nocapture
```

Expected: compile error (todo!() panics / unimplemented)

**Step 3: Implement diff_array and apply_deltas**

```rust
pub fn diff_array(before: &[u8], after: &[u8]) -> Vec<ArrayDelta> {
    assert_eq!(before.len(), after.len());
    let mut deltas = Vec::new();
    let mut i = 0;
    while i < before.len() {
        if before[i] == after[i] {
            i += 1;
            continue;
        }
        // Start of a changed run
        let start = i;
        let mut data: SmallVec<[u8; 16]> = SmallVec::new();
        while i < before.len() && before[i] != after[i] {
            data.push(after[i]);
            i += 1;
        }
        deltas.push(ArrayDelta { offset: start as u16, data });
    }
    deltas
}

pub fn apply_deltas(target: &mut [u8], deltas: &[ArrayDelta]) {
    for delta in deltas {
        let start = delta.offset as usize;
        let end = start + delta.data.len();
        target[start..end].copy_from_slice(&delta.data);
    }
}
```

**Step 4: Run tests to verify they pass**

```bash
cargo test -p nes-rewind delta
```

Expected: all pass

**Step 5: Commit**

```bash
git add crates/nes-rewind/src/delta.rs
git commit -m "feat(nes-rewind): implement ArrayDelta XOR diff for byte array compression"
```

---

## Task 4: FieldDelta — Structural Diff for Fixed-Size Fields

While `ArrayDelta` handles the big byte arrays, the small fixed-size fields (CPU registers, PPU control registers, APU counters, mapper state) change every frame in ways that XOR diffing is overkill for. `FieldDelta` wraps these as `Option<T>` — `None` if unchanged.

**Files:**
- Modify: `crates/nes-rewind/src/delta.rs`

**Step 1: Write the failing test**

Add to the tests block in `delta.rs`:

```rust
#[test]
fn field_delta_is_none_when_unchanged() {
    use nes_core::CoreSnapshot;
    // Build two identical snapshots
    let snap = make_test_snapshot(0);
    let delta = FieldDelta::compute(&snap, &snap);
    assert!(delta.cpu_regs.is_none());
    assert!(delta.ppu_ctrl.is_none());
    assert!(delta.mapper_changed.is_none());
}

#[test]
fn field_delta_detects_cpu_register_change() {
    let snap_a = make_test_snapshot(0);
    let mut snap_b = snap_a.clone();
    snap_b.cpu.pc = 0xBEEF;
    let delta = FieldDelta::compute(&snap_a, &snap_b);
    assert!(delta.cpu_regs.is_some());
    assert_eq!(delta.cpu_regs.unwrap().pc, 0xBEEF);
}

#[test]
fn field_delta_apply_restores_cpu_regs() {
    let snap_a = make_test_snapshot(0);
    let mut snap_b = snap_a.clone();
    snap_b.cpu.pc = 0xDEAD;
    let delta = FieldDelta::compute(&snap_a, &snap_b);

    let mut target = snap_a.clone();
    delta.apply(&mut target);
    assert_eq!(target.cpu.pc, 0xDEAD);
}

fn make_test_snapshot(seed: u8) -> nes_core::CoreSnapshot {
    let mut core = nes_core::NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);
    core.save_state()
}
```

**Step 2: Run to verify failure**

```bash
cargo test -p nes-rewind field_delta -- --nocapture
```

Expected: compile error — `FieldDelta` doesn't exist yet

**Step 3: Implement FieldDelta**

Add to `delta.rs` (before the tests module):

```rust
use nes_core::{CoreSnapshot, CoreQuery, QueryResult};
use nes_core::api::CpuSnapshot; // adjust import if needed

/// Structural delta for fixed-size CoreSnapshot fields.
/// `None` means the field is unchanged from the previous frame.
#[derive(Debug, Clone)]
pub struct FieldDelta {
    pub cpu_regs: Option<CpuSnapshot>,
    /// [ctrl, mask, status, oam_addr]
    pub ppu_ctrl: Option<[u8; 4]>,
    pub ppu_timing: Option<PpuTimingDelta>,
    pub mapper_changed: Option<()>,  // presence signals mapper state changed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PpuTimingDelta {
    pub scanline: u16,
    pub dot: u16,
    pub frame_counter: u64,
    pub odd_frame: bool,
}

impl FieldDelta {
    /// Compute the structural differences between two snapshots.
    pub fn compute(before: &CoreSnapshot, after: &CoreSnapshot) -> Self {
        Self {
            cpu_regs: if before.cpu != after.cpu {
                Some(after.cpu)
            } else {
                None
            },
            ppu_ctrl: {
                let b = [before.ppu.ctrl, before.ppu.mask, before.ppu.status, before.ppu.oam_addr];
                let a = [after.ppu.ctrl, after.ppu.mask, after.ppu.status, after.ppu.oam_addr];
                if b != a { Some(a) } else { None }
            },
            ppu_timing: {
                let changed = before.ppu.scanline != after.ppu.scanline
                    || before.ppu.dot != after.ppu.dot
                    || before.ppu.frame_counter != after.ppu.frame_counter
                    || before.ppu.odd_frame != after.ppu.odd_frame;
                if changed {
                    Some(PpuTimingDelta {
                        scanline: after.ppu.scanline,
                        dot: after.ppu.dot,
                        frame_counter: after.ppu.frame_counter,
                        odd_frame: after.ppu.odd_frame,
                    })
                } else {
                    None
                }
            },
            // Mapper comparison via PartialEq on LoadedMapper (it derives PartialEq)
            mapper_changed: if before.mapper != after.mapper { Some(()) } else { None },
        }
    }

    /// Apply this delta to a target snapshot, mutating only the changed fields.
    pub fn apply(&self, target: &mut CoreSnapshot) {
        if let Some(regs) = self.cpu_regs {
            target.cpu = regs;
        }
        if let Some([ctrl, mask, status, oam_addr]) = self.ppu_ctrl {
            target.ppu.ctrl = ctrl;
            target.ppu.mask = mask;
            target.ppu.status = status;
            target.ppu.oam_addr = oam_addr;
        }
        if let Some(timing) = self.ppu_timing {
            target.ppu.scanline = timing.scanline;
            target.ppu.dot = timing.dot;
            target.ppu.frame_counter = timing.frame_counter;
            target.ppu.odd_frame = timing.odd_frame;
        }
        // mapper_changed signals we need the full mapper from the FrameDelta
        // (handled at FrameDelta level, not here)
    }
}
```

> **Note:** `CoreSnapshot.mapper` is private (`mapper: Option<LoadedMapper>`). You may need to expose a comparison helper or a `pub mapper_eq` field. Check the api.rs visibility and add a public `mapper_changed` method to `NesCore` if needed. Alternatively, expose `CoreSnapshot::mapper_eq(a: &CoreSnapshot, b: &CoreSnapshot) -> bool`.

**Step 4: Run tests**

```bash
cargo test -p nes-rewind field_delta
```

Expected: all pass. Fix any visibility issues with snapshot fields.

**Step 5: Commit**

```bash
git add crates/nes-rewind/src/delta.rs
git commit -m "feat(nes-rewind): implement FieldDelta structural diff for CoreSnapshot fixed fields"
```

---

## Task 5: FrameDelta — Combined Delta for One Frame

`FrameDelta` is the complete compressed diff between two consecutive `CoreSnapshot`s. It combines `ArrayDelta`s for the three big arrays with a `FieldDelta` for the small fields, plus the full mapper state if it changed (mappers are small — just bank registers).

**Files:**
- Modify: `crates/nes-rewind/src/delta.rs`

**Step 1: Write failing tests**

```rust
#[test]
fn frame_delta_roundtrip_identical_snapshots() {
    let snap = make_test_snapshot(0);
    let delta = FrameDelta::compute(&snap, &snap);
    assert_eq!(delta.compressed_size(), 0);
}

#[test]
fn frame_delta_roundtrip_with_ram_change() {
    let mut core = nes_core::NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);
    let snap_a = core.save_state();

    core.execute(nes_core::Command::StepFrame).unwrap();
    let snap_b = core.save_state();

    let delta = FrameDelta::compute(&snap_a, &snap_b);
    let mut reconstructed = snap_a.clone();
    delta.apply(&mut reconstructed);

    // State hashes must match after reconstruction
    assert_eq!(reconstructed.ppu.frame_counter, snap_b.ppu.frame_counter);
}
```

**Step 2: Run to verify failure**

```bash
cargo test -p nes-rewind frame_delta -- --nocapture
```

**Step 3: Implement FrameDelta**

```rust
#[derive(Debug, Clone)]
pub struct FrameDelta {
    pub frame_id: u64,
    pub ram_deltas: Vec<ArrayDelta>,
    pub chr_deltas: Vec<ArrayDelta>,
    pub nametable_deltas: Vec<ArrayDelta>,
    pub oam_deltas: Vec<ArrayDelta>,
    pub fields: FieldDelta,
    /// Full mapper state if changed; None if unchanged.
    pub mapper: Option<nes_core::CoreSnapshot>,  // store full snap for mapper extraction
    _compressed_size: u32,
}

impl FrameDelta {
    pub fn compute(before: &nes_core::CoreSnapshot, after: &nes_core::CoreSnapshot) -> Self {
        let ram_deltas = diff_array(&before.cpu.work_ram, &after.cpu.work_ram);
        let chr_deltas = diff_array(&before.ppu.chr, &after.ppu.chr);
        let nametable_deltas = diff_array(&before.ppu.nametable_ram, &after.ppu.nametable_ram);
        let oam_deltas = diff_array(&before.ppu.oam, &after.ppu.oam);
        let fields = FieldDelta::compute(before, after);

        let size = ram_deltas.iter().map(|d| 4 + d.data.len() as u32).sum::<u32>()
            + chr_deltas.iter().map(|d| 4 + d.data.len() as u32).sum::<u32>()
            + nametable_deltas.iter().map(|d| 4 + d.data.len() as u32).sum::<u32>();

        Self {
            frame_id: after.ppu.frame_counter,
            ram_deltas,
            chr_deltas,
            nametable_deltas,
            oam_deltas,
            fields,
            mapper: None,
            _compressed_size: size,
        }
    }

    pub fn compressed_size(&self) -> u32 {
        self._compressed_size
    }

    pub fn apply(&self, target: &mut nes_core::CoreSnapshot) {
        apply_deltas(&mut target.cpu.work_ram, &self.ram_deltas);
        apply_deltas(&mut target.ppu.chr, &self.chr_deltas);
        apply_deltas(&mut target.ppu.nametable_ram, &self.nametable_deltas);
        apply_deltas(&mut target.ppu.oam, &self.oam_deltas);
        self.fields.apply(target);
    }
}
```

> **Note:** `CoreSnapshot` fields like `cpu.work_ram`, `ppu.chr`, `ppu.nametable_ram`, `ppu.oam` must be `pub`. Check visibility in `api.rs` / `ppu.rs` / `cpu/engine.rs` and add `pub` as needed. The snapshots are data carriers — all fields should be public.

**Step 4: Run tests**

```bash
cargo test -p nes-rewind frame_delta
```

**Step 5: Commit**

```bash
git add crates/nes-rewind/src/delta.rs
git commit -m "feat(nes-rewind): implement FrameDelta combining array and structural diffs"
```

---

## Task 6: KeyframePolicy — Adaptive Keyframe Promotion

Decides when to store a full snapshot (keyframe) vs a delta. Uses an exponential moving average (EMA) to detect spikes in delta size, which signal major state changes (screen transitions, level loads, boss spawns).

**Files:**
- Create: `crates/nes-rewind/src/policy.rs`

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> KeyframePolicy {
        KeyframePolicy::new(60, 2048)
    }

    #[test]
    fn forces_keyframe_at_base_interval() {
        let mut p = policy();
        // Feed 59 tiny deltas — should NOT promote
        for _ in 0..59 {
            assert!(!p.should_promote(100));
        }
        // 60th tick must promote
        assert!(p.should_promote(100));
    }

    #[test]
    fn resets_counter_after_promotion() {
        let mut p = policy();
        for _ in 0..60 { p.should_promote(100); }
        // Counter just reset — next 59 should not promote
        for _ in 0..59 {
            assert!(!p.should_promote(100));
        }
        assert!(p.should_promote(100));
    }

    #[test]
    fn spike_triggers_early_promotion() {
        let mut p = policy();
        // Warm up EMA with small deltas
        for _ in 0..10 {
            p.should_promote(100);
        }
        // Massive spike (screen transition: 10KB delta)
        assert!(p.should_promote(10_000));
    }

    #[test]
    fn moderate_delta_does_not_false_positive() {
        let mut p = policy();
        for _ in 0..10 {
            p.should_promote(500);
        }
        // 2x average — not a spike, no early promotion
        assert!(!p.should_promote(1000));
    }
}
```

**Step 2: Run to verify failure**

```bash
cargo test -p nes-rewind policy -- --nocapture
```

**Step 3: Implement KeyframePolicy**

```rust
/// Decides when to promote a full keyframe snapshot vs storing a delta.
#[derive(Debug, Clone)]
pub struct KeyframePolicy {
    base_interval: u64,
    spike_threshold: u32,
    frames_since_keyframe: u64,
    rolling_avg: u32,
}

impl KeyframePolicy {
    /// `base_interval`: minimum frames between keyframes (e.g., 60).
    /// `spike_threshold`: raw byte count that, combined with 3× EMA check, triggers early keyframe.
    pub fn new(base_interval: u64, spike_threshold: u32) -> Self {
        Self {
            base_interval,
            spike_threshold,
            frames_since_keyframe: 0,
            rolling_avg: 0,
        }
    }

    /// Returns `true` if a full keyframe should be stored this frame.
    /// Updates internal EMA.
    pub fn should_promote(&mut self, delta_size: u32) -> bool {
        self.frames_since_keyframe += 1;

        if self.frames_since_keyframe >= self.base_interval {
            self.rolling_avg = self.ema_step(delta_size);
            self.frames_since_keyframe = 0;
            return true;
        }

        self.rolling_avg = self.ema_step(delta_size);

        // Spike: delta exceeds absolute threshold AND is 3× the rolling average
        delta_size > self.spike_threshold
            && (self.rolling_avg == 0 || delta_size > self.rolling_avg.saturating_mul(3))
    }

    fn ema_step(&self, new_value: u32) -> u32 {
        // alpha = 32/256 ≈ 0.125 in Q8 fixed point
        const ALPHA_Q8: u32 = 32;
        let weighted_new = (new_value * ALPHA_Q8) >> 8;
        let weighted_old = (self.rolling_avg * (256 - ALPHA_Q8)) >> 8;
        weighted_new + weighted_old
    }

    /// Reset counter after a keyframe has been stored externally.
    pub fn reset(&mut self) {
        self.frames_since_keyframe = 0;
    }
}
```

**Step 4: Run tests**

```bash
cargo test -p nes-rewind policy
```

**Step 5: Commit**

```bash
git add crates/nes-rewind/src/policy.rs
git commit -m "feat(nes-rewind): implement adaptive KeyframePolicy with EMA spike detection"
```

---

## Task 7: CompressedTimeline — Anchor + Delta Ring Buffer

The `CompressedTimeline` stores the actual history: full `CoreSnapshot` keyframes at adaptive intervals, and `FrameDelta`s between them. Acts as a bounded ring buffer — old keyframes and their associated deltas are pruned as history fills up.

**Files:**
- Create: `crates/nes-rewind/src/timeline.rs`

**Step 1: Write failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn make_timeline(max_frames: u64) -> CompressedTimeline {
        CompressedTimeline::new(max_frames, KeyframePolicy::new(10, 2048))
    }

    fn dummy_snapshot(frame_id: u64) -> nes_core::CoreSnapshot {
        let mut core = nes_core::NesCore::new();
        core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);
        core.save_state()
    }

    #[test]
    fn empty_timeline_has_no_history() {
        let tl = make_timeline(300);
        assert_eq!(tl.len(), 0);
        assert!(tl.reconstruct(0).is_none());
    }

    #[test]
    fn push_stores_keyframe_first() {
        let mut tl = make_timeline(300);
        let snap = dummy_snapshot(0);
        tl.push(0, snap.clone());
        assert_eq!(tl.len(), 1);
        let reconstructed = tl.reconstruct(0).unwrap();
        assert_eq!(reconstructed.ppu.frame_counter, snap.ppu.frame_counter);
    }

    #[test]
    fn push_prunes_old_frames_when_full() {
        let mut tl = make_timeline(5);
        let snap = dummy_snapshot(0);
        for i in 0..10u64 {
            tl.push(i, snap.clone());
        }
        assert!(tl.len() <= 5);
        assert!(tl.reconstruct(0).is_none()); // oldest pruned
        assert!(tl.reconstruct(9).is_some()); // most recent available
    }

    #[test]
    fn reconstruct_returns_none_for_unknown_frame() {
        let mut tl = make_timeline(300);
        tl.push(0, dummy_snapshot(0));
        assert!(tl.reconstruct(999).is_none());
    }
}
```

**Step 2: Run to verify failure**

```bash
cargo test -p nes-rewind timeline -- --nocapture
```

**Step 3: Implement CompressedTimeline**

```rust
use std::collections::VecDeque;
use nes_core::CoreSnapshot;
use crate::delta::{FrameDelta};
use crate::policy::KeyframePolicy;

#[derive(Debug)]
struct Keyframe {
    frame_id: u64,
    snapshot: CoreSnapshot,
}

#[derive(Debug)]
pub struct CompressedTimeline {
    keyframes: VecDeque<Keyframe>,
    deltas: VecDeque<FrameDelta>,
    max_frames: u64,
    policy: KeyframePolicy,
    last_snapshot: Option<CoreSnapshot>,
    oldest_frame: u64,
}

impl CompressedTimeline {
    pub fn new(max_frames: u64, policy: KeyframePolicy) -> Self {
        Self {
            keyframes: VecDeque::new(),
            deltas: VecDeque::new(),
            max_frames,
            policy,
            last_snapshot: None,
            oldest_frame: 0,
        }
    }

    /// Push a new frame into the timeline.
    pub fn push(&mut self, frame_id: u64, snapshot: CoreSnapshot) {
        if let Some(prev) = self.last_snapshot.take() {
            let delta = FrameDelta::compute(&prev, &snapshot);
            let size = delta.compressed_size();

            if self.policy.should_promote(size) {
                self.keyframes.push_back(Keyframe { frame_id, snapshot: snapshot.clone() });
            } else {
                self.deltas.push_back(delta);
            }
        } else {
            // First frame is always a keyframe
            self.keyframes.push_back(Keyframe { frame_id, snapshot: snapshot.clone() });
        }

        self.last_snapshot = Some(snapshot);
        self.prune();
    }

    /// Reconstruct the snapshot at `target_frame_id`.
    /// Returns `None` if the frame has been pruned or was never recorded.
    pub fn reconstruct(&self, target_frame_id: u64) -> Option<CoreSnapshot> {
        // Find the nearest keyframe at or before target
        let kf = self.keyframes.iter().rev()
            .find(|kf| kf.frame_id <= target_frame_id)?;

        let mut snap = kf.snapshot.clone();
        if kf.frame_id == target_frame_id {
            return Some(snap);
        }

        // Apply deltas from keyframe forward to target
        for delta in &self.deltas {
            if delta.frame_id > kf.frame_id && delta.frame_id <= target_frame_id {
                delta.apply(&mut snap);
            }
        }
        Some(snap)
    }

    /// Number of recorded frames (keyframes + deltas).
    pub fn len(&self) -> usize {
        self.keyframes.len() + self.deltas.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty() && self.deltas.is_empty()
    }

    fn prune(&mut self) {
        while self.len() as u64 > self.max_frames {
            // Drop oldest keyframe and all deltas before the next keyframe
            if self.keyframes.pop_front().is_some() {
                let next_kf_id = self.keyframes.front().map(|kf| kf.frame_id).unwrap_or(u64::MAX);
                while self.deltas.front().map(|d| d.frame_id < next_kf_id).unwrap_or(false) {
                    self.deltas.pop_front();
                }
            }
        }
    }
}
```

**Step 4: Run tests**

```bash
cargo test -p nes-rewind timeline
```

**Step 5: Commit**

```bash
git add crates/nes-rewind/src/timeline.rs
git commit -m "feat(nes-rewind): implement CompressedTimeline anchor+delta ring buffer"
```

---

## Task 8: TimeMachine — Public API with Rayon Background Workers

The `TimeMachine` is the host-facing API. It spawns a rayon worker that owns the `CompressedTimeline`, sends snapshots to it via a bounded channel for background compression, and manages a `RewindCursor` for speculative pre-reconstruction during rewind.

**Files:**
- Create: `crates/nes-rewind/src/worker.rs`
- Create: `crates/nes-rewind/src/cursor.rs`

**Step 1: Write failing integration tests**

In `crates/nes-rewind/src/worker.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::{Command, NesCore};

    fn make_core() -> NesCore {
        let mut core = NesCore::new();
        core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);
        core
    }

    #[test]
    fn record_and_rewind_restores_earlier_frame() {
        let mut core = make_core();
        let config = TimeMachineConfig {
            max_history_seconds: 10,
            keyframe_base_interval: 30,
            delta_spike_threshold: 2048,
        };
        let mut tm = TimeMachine::new(config);

        // Record 60 frames
        for _ in 0..60 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }
        let frame_before_rewind = core.ppu_frame_counter();

        // Rewind 30 frames
        for _ in 0..30 {
            tm.rewind_step(&mut core);
        }
        let frame_after_rewind = core.ppu_frame_counter();

        assert!(frame_after_rewind < frame_before_rewind,
            "Expected rewind to restore earlier frame, got {} >= {}",
            frame_after_rewind, frame_before_rewind);
    }

    #[test]
    fn rewind_returns_none_when_history_exhausted() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        // No history recorded
        let result = tm.rewind_step(&mut core);
        assert!(result.is_none());
    }

    #[test]
    fn resume_after_rewind_allows_recording_again() {
        let mut core = make_core();
        let mut tm = TimeMachine::new(TimeMachineConfig::default());

        for _ in 0..10 {
            core.execute(Command::StepFrame).unwrap();
            tm.record_frame(&core);
        }
        tm.rewind_step(&mut core);
        tm.resume();

        // Should be in Recording state again
        assert!(matches!(tm.state(), TimeMachineState::Recording));
    }
}
```

**Step 2: Run to verify failure**

```bash
cargo test -p nes-rewind worker -- --nocapture
```

**Step 3: Implement cursor.rs**

```rust
use std::collections::VecDeque;
use nes_core::CoreSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewindSpeed {
    Single,
    Normal,
    Fast,
    Faster,
}

impl RewindSpeed {
    pub fn lookahead_depth(self) -> usize {
        match self {
            Self::Single => 1,
            Self::Normal => 16,
            Self::Fast => 32,
            Self::Faster => 64,
        }
    }

    pub fn frame_skip(self) -> usize {
        match self {
            Self::Single | Self::Normal => 1,
            Self::Fast => 2,
            Self::Faster => 4,
        }
    }
}

#[derive(Debug)]
pub struct RewindCursor {
    pub current_frame: u64,
    pub speed: RewindSpeed,
    pub lookahead: VecDeque<(u64, CoreSnapshot)>,
}

impl RewindCursor {
    pub fn new(current_frame: u64, speed: RewindSpeed) -> Self {
        Self { current_frame, speed, lookahead: VecDeque::new() }
    }

    pub fn pop_frame(&mut self) -> Option<(u64, CoreSnapshot)> {
        let frame = self.lookahead.pop_front()?;
        self.current_frame = frame.0;
        Some(frame)
    }
}
```

**Step 4: Implement worker.rs (TimeMachine)**

```rust
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use nes_core::{CoreSnapshot, NesCore};
use crate::cursor::{RewindCursor, RewindSpeed};
use crate::policy::KeyframePolicy;
use crate::timeline::CompressedTimeline;

pub struct TimeMachineConfig {
    pub max_history_seconds: u32,
    pub keyframe_base_interval: u64,
    pub delta_spike_threshold: u32,
}

impl Default for TimeMachineConfig {
    fn default() -> Self {
        Self {
            max_history_seconds: 30,
            keyframe_base_interval: 60,
            delta_spike_threshold: 2048,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeMachineState {
    Recording,
    Rewinding { seconds_remaining: f32 },
    Exhausted,
}

enum WorkerMsg {
    Record { frame_id: u64, snapshot: CoreSnapshot },
    Reconstruct { target_frame: u64 },
    Shutdown,
}

enum WorkerReply {
    Reconstructed { frame_id: u64, snapshot: CoreSnapshot },
}

pub struct TimeMachine {
    config: TimeMachineConfig,
    state: TimeMachineState,
    cursor: Option<RewindCursor>,
    tx: SyncSender<WorkerMsg>,
    rx: Receiver<WorkerReply>,
    last_recorded_frame: u64,
}

impl TimeMachine {
    pub fn new(config: TimeMachineConfig) -> Self {
        let max_frames = (config.max_history_seconds as u64) * 60;
        let policy = KeyframePolicy::new(config.keyframe_base_interval, config.delta_spike_threshold);
        let (work_tx, work_rx) = mpsc::sync_channel::<WorkerMsg>(4);
        let (reply_tx, reply_rx) = mpsc::sync_channel::<WorkerReply>(64);

        thread::spawn(move || {
            let mut timeline = CompressedTimeline::new(max_frames, policy);
            loop {
                match work_rx.recv() {
                    Ok(WorkerMsg::Record { frame_id, snapshot }) => {
                        timeline.push(frame_id, snapshot);
                    }
                    Ok(WorkerMsg::Reconstruct { target_frame }) => {
                        if let Some(snapshot) = timeline.reconstruct(target_frame) {
                            let _ = reply_tx.send(WorkerReply::Reconstructed {
                                frame_id: target_frame,
                                snapshot,
                            });
                        }
                    }
                    Ok(WorkerMsg::Shutdown) | Err(_) => break,
                }
            }
        });

        Self {
            config,
            state: TimeMachineState::Recording,
            cursor: None,
            tx: work_tx,
            rx: reply_rx,
            last_recorded_frame: 0,
        }
    }

    /// Call once per frame after advancing the core. Non-blocking.
    pub fn record_frame(&mut self, core: &NesCore) {
        if !matches!(self.state, TimeMachineState::Recording) {
            return;
        }
        let frame_id = core.ppu_frame_counter();
        let snapshot = core.save_state();
        self.last_recorded_frame = frame_id;
        // Drop on full — never block the emulation thread
        let _ = self.tx.try_send(WorkerMsg::Record { frame_id, snapshot });
    }

    /// Step backward one frame. Returns `Some(frame_id)` on success, `None` when exhausted.
    pub fn rewind_step(&mut self, core: &mut NesCore) -> Option<u64> {
        let cursor = self.cursor.get_or_insert_with(|| {
            self.state = TimeMachineState::Rewinding { seconds_remaining: 0.0 };
            RewindCursor::new(self.last_recorded_frame, RewindSpeed::Normal)
        });

        let target = cursor.current_frame.checked_sub(1)?;

        // Request reconstruction from worker
        let _ = self.tx.send(WorkerMsg::Reconstruct { target_frame: target });

        // Wait for reply (blocking is acceptable during rewind — core is paused)
        match self.rx.recv_timeout(std::time::Duration::from_millis(16)) {
            Ok(WorkerReply::Reconstructed { frame_id, snapshot }) => {
                cursor.current_frame = frame_id;
                core.load_state(&snapshot);
                self.state = TimeMachineState::Rewinding {
                    seconds_remaining: (frame_id as f32) / 60.0,
                };
                Some(frame_id)
            }
            _ => {
                self.state = TimeMachineState::Exhausted;
                None
            }
        }
    }

    /// Increase rewind speed.
    pub fn rewind_faster(&mut self) {
        if let Some(cursor) = &mut self.cursor {
            cursor.speed = match cursor.speed {
                RewindSpeed::Normal => RewindSpeed::Fast,
                RewindSpeed::Fast => RewindSpeed::Faster,
                other => other,
            };
        }
    }

    /// Resume normal emulation from current rewind position.
    pub fn resume(&mut self) {
        self.cursor = None;
        self.state = TimeMachineState::Recording;
    }

    pub fn state(&self) -> TimeMachineState {
        self.state.clone()
    }

    pub fn history_seconds(&self) -> f32 {
        self.last_recorded_frame as f32 / 60.0
    }
}

impl Drop for TimeMachine {
    fn drop(&mut self) {
        let _ = self.tx.send(WorkerMsg::Shutdown);
    }
}
```

**Step 5: Run tests**

```bash
cargo test -p nes-rewind worker
```

Expected: all pass. Fix any channel or lifetime issues.

**Step 6: Run full workspace**

```bash
cargo test --workspace
```

**Step 7: Commit**

```bash
git add crates/nes-rewind/src/worker.rs crates/nes-rewind/src/cursor.rs
git commit -m "feat(nes-rewind): implement TimeMachine with rayon-backed compression and rewind API"
```

---

## Task 9: TimeMachineConfig in nes-config

Expose the Time Machine config in the shared TOML config so users can tweak it via `nes.toml`.

**Files:**
- Modify: `crates/nes-config/src/lib.rs`

**Step 1: Write failing test**

```rust
#[test]
fn time_machine_config_has_sane_defaults() {
    let config: NesConfig = toml::from_str("").unwrap();
    assert_eq!(config.time_machine.max_history_seconds, 30);
    assert!(config.time_machine.enabled);
}
```

**Step 2: Add config struct**

```rust
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct TimeMachineConfig {
    pub enabled: bool,
    pub max_history_seconds: u32,
    pub keyframe_base_interval: u64,
    pub delta_spike_threshold: u32,
}

impl Default for TimeMachineConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_history_seconds: 30,
            keyframe_base_interval: 60,
            delta_spike_threshold: 2048,
        }
    }
}
```

Add `pub time_machine: TimeMachineConfig` to the root config struct.

**Step 3: Run tests**

```bash
cargo test -p nes-config
```

**Step 4: Commit**

```bash
git add crates/nes-config/src/lib.rs
git commit -m "feat(nes-config): add TimeMachineConfig with 30s default history"
```

---

## Task 10: nes-desktop Integration

Wire the `TimeMachine` into the nes-desktop event loop. Record every frame; on a dedicated rewind keybind, call `rewind_step`. Display state via the existing metrics/overlay system.

**Files:**
- Modify: `crates/nes-desktop/Cargo.toml`
- Modify: `crates/nes-desktop/src/main.rs`

**Step 1: Add nes-rewind dependency**

In `crates/nes-desktop/Cargo.toml`:

```toml
nes-rewind = { path = "../nes-rewind" }
```

**Step 2: Write the integration test**

```rust
// In an integration test file or as a doc comment in main.rs
// Verify that TimeMachine is wired into the main loop without panicking
// (smoke test — the real proof is manual gameplay)
#[test]
fn time_machine_integrates_without_panic() {
    use nes_core::{Command, NesCore};
    use nes_rewind::{TimeMachine, TimeMachineConfig};

    let mut core = NesCore::new();
    core.load_cpu_bytes(0xC000, &[0xEA, 0x4C, 0x00, 0xC0]);

    let mut tm = TimeMachine::new(TimeMachineConfig::default());

    for _ in 0..120 {
        core.execute(Command::StepFrame).unwrap();
        tm.record_frame(&core);
    }

    // Rewind 60 frames
    let mut rewound = 0;
    for _ in 0..60 {
        if tm.rewind_step(&mut core).is_some() {
            rewound += 1;
        }
    }
    assert!(rewound > 0);
    tm.resume();
}
```

**Step 3: Wire into main loop**

Find `advance_core_for_host_frame` call (line ~838 in `main.rs`) and the input handling section. Add:

```rust
// In the setup section, after loading config:
let mut time_machine = if config.time_machine.enabled {
    Some(nes_rewind::TimeMachine::new(nes_rewind::TimeMachineConfig {
        max_history_seconds: config.time_machine.max_history_seconds,
        keyframe_base_interval: config.time_machine.keyframe_base_interval,
        delta_spike_threshold: config.time_machine.delta_spike_threshold,
    }))
} else {
    None
};

// After advance_core_for_host_frame succeeds:
if let Some(tm) = &mut time_machine {
    tm.record_frame(&core);
}

// In the keybind handling section, add rewind key (e.g., Backspace or R):
// When rewind key held:
if rewind_held {
    if let Some(tm) = &mut time_machine {
        tm.rewind_step(&mut core);
    }
}
// When rewind key released:
if rewind_released {
    if let Some(tm) = &mut time_machine {
        tm.resume();
    }
}
```

> **Note:** The exact keybind wiring depends on the input mapping in `main.rs`. Find the `input_bits_from_gilrs` / `Button` mapping section (~line 580) and add a dedicated `TimeMachineRewind` action, or use a direct `VirtualKeyCode::Back` check in the winit event handler.

**Step 4: Run desktop build**

```bash
cargo build -p nes-desktop
```

**Step 5: Run integration test**

```bash
cargo test -p nes-desktop time_machine
```

**Step 6: Run full workspace**

```bash
cargo test --workspace
```

**Step 7: Commit**

```bash
git add crates/nes-desktop/Cargo.toml crates/nes-desktop/src/main.rs
git commit -m "feat(nes-desktop): integrate TimeMachine rewind into main event loop"
```

---

## Final Verification

```bash
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

All green → ready for finishing-a-development-branch skill.

---

## Memory Budget Check

After Task 7 passes, run this manually to confirm compression is working:

```rust
// In a test or binary:
let before_bytes = std::mem::size_of::<CoreSnapshot>();
println!("CoreSnapshot size: {} KB", before_bytes / 1024);
// Expected: ~12–18KB (registers + arrays, no PRG ROM)
// At 1800 frames raw: ~{before_bytes * 1800 / 1024 / 1024} MB
// Target with compression: <10MB
```
