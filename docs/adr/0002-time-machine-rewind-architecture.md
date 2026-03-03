# ADR 0002: Time Machine — Anchor+Delta Rewind Architecture

Date: 2026-03-02
Status: Accepted

## Context

Players need a rewind ("Time Machine") feature that lets them step backward through the last ~30 seconds of gameplay and resume forward play from any point. The NES emulator state is large (~13 KB per `CoreSnapshot`) and the emulation loop runs at 60 fps, making naive full-snapshot storage impractical:

- 30 s × 60 fps × 13 KB ≈ **23 MB** of raw snapshot data
- Allocation jitter from per-frame heap snapshots would cause audio dropout

Additionally, the desktop rendering path has a subtle correctness requirement: when a historical snapshot is *restored*, the PPU must re-render the full framebuffer from a static, single-scroll position — unlike live play, where the scroll changes mid-frame via the sprite-0-hit trick.

## Decision

### 1. Anchor + Delta compression in `nes-rewind`

Store a **keyframe** (full `CoreSnapshot`) every N frames, with **delta frames** between them:

- `ArrayDelta` — run-length encoded byte diffs for large arrays (CHR, nametable RAM, OAM, palette RAM, work RAM).
- `FieldDelta` — `Option`-per-field diff for scalar registers (CPU, PPU control, PPU timing, PPU scroll/Loopy registers).
- `FrameDelta` — combines all array and field deltas for one inter-keyframe step.
- `CompressedTimeline` — a `VecDeque` of `(keyframe, Vec<FrameDelta>)` segments; prunes oldest segment when over capacity.

Keyframe promotion is governed by `KeyframePolicy`:

- Forced every `keyframe_base_interval` frames (default: 60).
- Early promotion when delta byte count spikes past `delta_spike_threshold × 3× EMA`, signalling a scene change where deltas would be large anyway.

Reconstruction walks forward from the nearest keyframe ≤ target, applying deltas in order — O(N) in the number of inter-keyframe deltas, bounded by `keyframe_base_interval`.

### 2. Background worker thread

All compression and reconstruction runs on a dedicated thread (`nes-rewind::worker`):

- `record_frame()` — `try_send` (non-blocking); drops if worker channel is full rather than stalling the audio/video loop.
- `rewind_step()` — blocking send + `recv_timeout(16 ms)`; acceptable because the emulation loop is paused during rewind.
- Channel capacities: work channel = 4 (small, drops gracefully), reply channel = 64 (burst-tolerant).

### 3. VBlank scroll capture to fix the sprite-0-hit rendering split

SMB (and many NES games) split the screen using the **sprite-0-hit** trick:

1. The NMI handler fires at VBlank (scanline 241) and writes `scroll_x = 0` (for the status bar).
2. Mid-frame, at sprite-0 hit, the game writes the real level scroll (e.g., 200) to `$2005`.
3. The PPU picks up both writes dynamically during dot-by-dot rendering — live play looks correct.

Snapshots are captured after `step_until_next_frame()` (frame counter increment = scanline 0 of the *next* frame). By that point the NMI handler has already run, so `scroll_x` in the snapshot holds the **status-bar** scroll (≈ 0), not the game-area scroll.

When `restore()` calls `render_full_framebuffer()`, it rendered the whole screen at scroll 0, making pipes and scenery jump to the wrong horizontal position.

**Fix:** `Ppu` captures `render_scroll_x` and `render_ctrl` at the *exact dot* VBlank begins (scanline 241, dot 1) — before `nmi_pending` is even set, so before the CPU can execute any NMI handler code. `render_full_framebuffer()` temporarily substitutes these captured values for the static re-render.

Both fields are included in `PpuSnapshot` and tracked in `PpuScrollDelta` so delta-reconstructed frames between keyframes also render with the correct scroll.

### 4. Controller release on resume

When rewinding, `load_state()` restores historical `controller_bits`. On resume, the current physical button state is correct, but the core's internal latch still holds stale bits from the snapshot. Fix: after `time_machine.resume()`, fire `Command::ReleaseButton` for all eight buttons unconditionally.

## Architecture diagram

```
┌─────────────────────────────────────────────────────────┐
│ nes-desktop (emulation loop)                            │
│                                                         │
│  StepFrame → record_frame() ──try_send──► Worker       │
│                                           │  push()    │
│  Rewind key → rewind_step() ──send──────► │ reconstruct│
│               ◄── recv_timeout(16ms) ─────┘            │
│                                                         │
│  Resume → ReleaseButton × 8 → Recording state          │
└─────────────────────────────────────────────────────────┘

CompressedTimeline layout:
  [KF₀ | Δ₁ Δ₂ … Δ₅₉] [KF₆₀ | Δ₆₁ … Δ₁₁₉] … [KFₙ | Δ…]
   oldest ─────────────────────────────────────── newest
   pruned when over max_frames capacity

PpuSnapshot timing (per frame):
  scanline 0:   frame_counter increments, NMI handler has run
  scanline 241: VBlank flag set → render_scroll_x/ctrl captured HERE
                → nmi_pending set → CPU runs NMI handler
                → handler writes scroll_x = 0 (status bar)
```

## Consequences

**Positive:**
- Memory footprint: ~23 MB worst-case raw → typically 2–4 MB compressed (SMB side-scrolling ~3% delta rate for CHR/nametable).
- Emulation loop never blocks on rewind I/O.
- Background glitch (sprite-0-hit scroll mismatch) eliminated for all games using mid-frame scroll splits.
- Controller state desync on resume eliminated.

**Tradeoffs:**
- Static re-render is a single-scroll approximation; games with more than one mid-frame scroll split per axis will not render pixel-perfectly during paused rewind preview. Forward play is unaffected.
- `rewind_step()` blocks up to 16 ms waiting for reconstruction; if the worker is behind (e.g., large CHR delta segment), a frame may be dropped silently.
- Early keyframe promotion heuristic (EMA spike) is tunable but not formally proven optimal; pathological content could still produce large segments.
