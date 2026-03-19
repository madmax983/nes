# ADR 0003: NES-Core Hot-Path Optimization — Removing Duplicate PPU/APU Sync

**Date:** 2026-03-18
**Status:** Accepted
**Context:** `crates/nes-core`

---

## Context

`nes-core` ran at approximately **3.5x real-time** (one 60Hz frame = 4.682ms median, real-time budget = 16.67ms). For AI training, rewind, and netplay rollback, throughput matters more than real-time — 10x+ is the target.

The Abrash Method was applied: baseline first, profile the hot path, one change at a time.

---

## Baseline Measurements

Established with Criterion-style benchmarks via `divan` (see `benches/frame_throughput.rs`):

| Benchmark | Fastest | Median | Real-time ratio |
|-----------|---------|--------|-----------------|
| `step_frame_warm` | 3.855ms | 4.682ms | 3.6x |
| `step_frame_cold` | 4.348ms | 4.846ms | 3.4x |
| `step_scanline_warm` | 16.29µs | 17.04µs | — |
| `step_cpu_nop` | 362.4ns | 368.6ns | — |
| `sixty_frames_burst` | 166.6ms | 291.7ms | 3.4x |

**Test ROM:** `NOP ; JMP $C000` — infinite NOP loop, exercises full CPU/PPU/APU pipeline.

---

## Hot-Path Analysis

### What `step_single_instruction` does per CPU instruction

```
sync_ppu_register_image()        ← 8 writes to CPU flat memory
cpu.step_with_trace_and_cycles() ← decode + execute + bus trace
apply_cpu_writes()               ← handle MMIO writes
apply_cpu_reads()                ← handle MMIO read side effects
advance_hardware_cycles(N)       ← N APU ticks + 3N PPU dots
NMI/IRQ check
sync_ppu_register_image()        ← DUPLICATE — same work as next iteration's start
```

The function called `sync_ppu_register_image()` **twice per instruction**: once before the CPU step (to present current state to the CPU) and once after `advance_hardware_cycles` (to update the image for the next instruction).

### Why the duplicate sync was redundant

`step_until_next_frame` calls `step_single_instruction` in a tight loop. The post-step sync at the end of iteration N is exactly equivalent to the pre-step sync at the start of iteration N+1 — they read the same PPU/APU state. The image is never read between the end of one instruction and the start of the next, so the post-step sync prepares state that is immediately overwritten.

### Cost of one sync call

- 8 `cpu.write_byte()` calls to the CPU flat memory array (`[u8; 65536]`)
- The PPU MMIO region ($2000–$4017) occupies different cache lines from the active code ($C000)
- Per step_cpu_nop measurement: eliminating one sync saved **~123ns**
- Per step_frame_warm measurement (in tight loop): **~55ns per instruction**

With ~35,736 instructions per frame: 35,736 × 55ns ≈ **2.0ms saved per frame**.

---

## Changes Made

### 1. Batch scheduler counter advances (`scheduler.rs`)

Added `Scheduler::advance_by(cpu_cycles)` which updates cpu/ppu/apu counters in a single call:

```rust
pub fn advance_by(&mut self, cpu_cycles: u64) {
    self.cpu_cycles = self.cpu_cycles.wrapping_add(cpu_cycles);
    self.ppu_cycles = self.ppu_cycles.wrapping_add(cpu_cycles * 3);
    self.apu_cycles = self.apu_cycles.wrapping_add(cpu_cycles);
}
```

`advance_hardware_cycles` uses this instead of 5 per-cycle increment calls.
**Measured impact:** Negligible — LLVM already inlined the trivial increment functions at `opt-level=3`.

### 2. Removed duplicate `sync_ppu_register_image()` call (`api.rs`)

Removed the post-step sync at the end of `step_single_instruction`. Added a comment explaining the invariant:

```rust
// Sync before the CPU step so reads during execution see current PPU/APU state.
// No post-step sync is needed: the next call's pre-step sync will refresh the
// image before the CPU runs again, and nothing reads the image between iterations.
self.sync_ppu_register_image();
```

**Measured impact:** ~1.97ms/frame reduction (42% of frame time).

### 3. Removed dead `step_hardware_cycle` function

After `advance_hardware_cycles` was refactored to inline the loop body, `step_hardware_cycle` became unused. Removed.

---

## Results

| Benchmark | Before | After | Speedup |
|-----------|--------|-------|---------|
| `step_frame_warm` (median) | 4.682ms | 2.714ms | **1.73x** |
| `step_frame_cold` (median) | 4.846ms | 2.742ms | **1.77x** |
| `step_scanline_warm` (median) | 17.04µs | 9.999µs | **1.70x** |
| `step_cpu_nop` (median) | 368.6ns | 245.2ns | **1.50x** |
| `sixty_frames_burst` (median) | 291.7ms | 170.6ms | **1.71x** |

**Real-time ratio:** 3.5x → **6.1x** (single frame), 3.4x → **5.9x** (60-frame burst).

All 103 unit tests + 27 doc tests pass.

---

## Lessons

1. **Measure before optimizing.** The scheduler batch change (hypothesis: 5–10% gain) showed zero improvement because LLVM already optimized it. The duplicate sync (hypothesis: maybe 5% gain) showed 70% improvement.

2. **Duplicated work is expensive.** `sync_ppu_register_image` was called 71,472 times per frame (twice × ~35,736 instructions). Each call touches a cold-ish cache region, accumulating to ~2ms of overhead.

3. **The cache miss pattern matters.** The CPU flat memory array is 64KB — larger than L1. The PPU MMIO region ($2000–$4017) sits far from the active code region ($C000), causing cache pressure on every sync call.

4. **Test-driven safety.** The optimization required understanding the invariant "nothing reads the CPU memory image between end-of-step-N and start-of-step-N+1." All existing tests validated this invariant held, with zero test changes needed.

---

---

## Optimization 3: Disable CPU Trace (2026-03-19)

**Hypothesis:** Skipping `format_trace` String allocations and bus-trace `RefCell::borrow_mut()` per `read()` call in throughput modes would save 5–15%.

**Actual result:** **4.0x speedup** — trace was consuming ~75% of per-instruction time.

### What the trace does per instruction

```
bus_trace.borrow_mut().clear()           ← RefCell borrow
read(pc) → record_bus_access()           ← RefCell borrow + Vec::push per read
... (2-6 reads/writes per instruction)
format_trace(snapshot, bytes, mnemonic)  ← format!() String allocation (~75 chars)
pad_microphase_to_cycle_count()          ← extra RefCell borrows + pushes
swap_bus_trace()                         ← Vec pointer swap
```

For a NOP (2 cycles): 2 `RefCell::borrow_mut()` + 2 `Vec::push(CpuBusAccess{4 bytes})` + 1 `format!()` heap alloc.

At ~35,736 instructions per frame that's ~35K String allocations + ~71K RefCell borrows per frame. String allocations were the dominant cost — each one allocates, writes ~75 chars, then is immediately replaced (triggering a free) on the next instruction.

### Change

Added `trace_enabled: bool` to `Cpu` (default `true`). When `false`:
- `record_bus_access()` is a no-op (guards in `read()`, `push()`, `write_and_track()`)
- `bus_trace.borrow_mut().clear()` skipped at start of step
- `pad_microphase_to_cycle_count()` skipped at end of step
- `maybe_trace()` returns `String::new()` (zero allocation) instead of formatting

`NesCore::set_trace_enabled(bool)` propagates to the CPU.

### Results

| Benchmark | Traced | No-trace | Speedup |
|-----------|--------|----------|---------|
| `step_frame_warm` (median) | 2.683ms | 671µs | **4.0x** |
| `step_cpu_nop` (median) | 229.5ns | 58.3ns | **3.9x** |
| `sixty_frames_burst` (median) | 164.4ms | 41.44ms | **3.97x** |

**No-trace real-time:** 671µs/frame → **24.8x real-time** (budget: 16.67ms).
**Cumulative from original baseline (4.682ms):** **6.98x total speedup**.

### Lessons

5. **Tracing overhead dominated emulation work.** In the traced path, ~75% of per-instruction time was String allocation and RefCell borrow overhead — not actual CPU/PPU/APU logic. The emulator was more of a trace-formatter than an emulator.

6. **Runtime toggle is better than compile-time flag.** Frontends can switch modes: trace=true for debugging/disassembly views, trace=false for AI training/rewind/netplay. No rebuild required.

7. **`String::new()` is zero-allocation.** The `maybe_trace()` helper returns `String::new()` in the disabled path — this is a stack-only empty vector (no heap alloc). The `is_empty()` check in `api.rs` distinguishes "disabled" from "traced" without a separate bool.

---

## Next Candidates

| Hypothesis | Estimate | Risk |
|------------|----------|------|
| Lazy sync: only call when CPU is about to read MMIO | 5–15% gain | Medium — requires MMIO read prediction |
| PPU scanline-level rendering (batch 256 pixels) | 10–25% gain | High — large refactor |
| APU timer batching (skip timer decrements between events) | 3–8% gain | Medium — frame sequencer coupling |
