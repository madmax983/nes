# ADR 0004: Embedded Target — `no_std` Support and Memory Architecture

Date: 2026-09-03
Status: Accepted
Context: `crates/nes-core`

## Context

We want `nes-core` to run on an ESP32-class microcontroller. That target is
defined by two hard limits the hosted builds never encounter:

- **No `std`.** Bare-metal targets have `core` and (with an allocator) `alloc`.
- **RAM measured in hundreds of kilobytes.** An ESP32-S3 has 512KB of SRAM
  total, of which roughly 320-400KB is usable after the vendor stack. An
  ESP32-P4 has 768KB.

Neither was true of the core as it stood.

### Measurements taken before any change

All numbers are from this repository, not estimates.

**`no_std` readiness.** Compiling the crate with `#![no_std] + extern crate
alloc` and `serde` set to `default-features = false` produced **199 errors**.
After a mechanical pass (`std::` → `core::`/`alloc::`, explicit `alloc`
imports), **14** remained, and all 14 were in four places: `ppm` (needs
`std::io::Write`), the APU mixer tables (`OnceLock`), the `nova` experimental
modules (`HashMap`/`HashSet`), and duplicate imports. Nothing in the CPU, PPU,
scheduler, or mapper layer required `std`.

**Memory.** With a counting global allocator, a running core holding a 16KB
NROM cartridge:

| Allocation | Bytes |
|---|---|
| `Cpu.memory` — flat `[u8; 0x10000]` address space (in `size_of::<NesCore>()`) | 65,536 |
| PPU framebuffer, RGBA | 245,760 |
| APU sample `VecDeque` — one full second of audio, doubling to 2x on overflow | 176,400 |
| APU TND mixer lookup table — `vec![0.0f32; 32768]`, built lazily on first sample | 131,072 |
| Mapper's own PRG copy | 16,384 |
| PPU `live_chr` | 8,192 |

`size_of::<NesCore>()` was 77,648 bytes and live heap was 489,839 bytes for
that cartridge — **~567KB total**, which does not fit an ESP32-S3 for *any*
ROM, before accounting for a display driver or network stack.

**Throughput.** A frame costs 3.08ms on a 2.10GHz Xeon: 6.47M host cycles for
89,342 PPU dots, or **~72 cycles per dot on a superscalar out-of-order core**.
An ESP32-S3 at 240MHz has 4.0M cycles per frame, a budget of **44.8 in-order
cycles per dot** for CPU, PPU, APU and mapper combined.

## Decision

### 1. `no_std` + `alloc` behind a default-on `std` feature

`nes-core` is `#![cfg_attr(not(feature = "std"), no_std)]` and always links
`alloc`. `alloc` types are imported explicitly per module, which resolves
identically under both configurations, so there is one code path rather than
two.

- `std` (default) enables `serde/std` and the `ppm` encoder.
- `nova` implies `std` — the experimental tooling uses hash collections and
  image encoders, and is not a target for embedded builds.
- `tas` works in both configurations.

Hosted consumers see no change: default features keep every existing API.

`core::error::Error` (stable since 1.81) replaced `std::error::Error`, so the
error types keep their trait implementations in both configurations.

The unused `arbitrary` dependency was removed. It was declared in
`[dependencies]` but referenced nowhere in the crate or the fuzz targets.

CI builds `--no-default-features` for `riscv32imc-unknown-none-elf` on every
push, so the guarantee is enforced rather than aspirational. That target is an
ESP32-C3/C6-class part and ships with upstream `rustc`. Xtensa parts (ESP32,
S2, S3) need the `esp-rs` toolchain fork; any `no_std` break shows up on the
RISC-V target first, so it serves as the gate for both.

### 2. Framebuffer stores palette indices, not RGBA

The PPU stores one 6-bit palette index per pixel (61,440 bytes) instead of four
RGBA bytes (245,760). `render_rgba` expands through a 256-entry lookup table
whose entries above `0x3F` are opaque black, so the "nothing rendered here yet"
sentinel costs no branch in the expansion loop.

This also shortens the per-dot render path from four byte stores to one, which
matters against the per-dot budget above.

Save states are unaffected: the framebuffer was never part of `PpuSnapshot`; it
is re-derived by `render_full_framebuffer()` on load.

### 3. APU mixer tables are compile-time constants

The pulse and TND mixer tables are `const fn`-evaluated into `static` arrays
rather than built lazily on the heap behind a `OnceLock`. 131KB of heap becomes
read-only data — RAM on a hosted target, flash on an embedded one — and the
per-sample `OnceLock` check disappears. Output is bit-identical.

### 4. APU sample backlog is capped in frames, not seconds

The cap dropped from 44,100 samples (one second, ~176KB once the deque doubled)
to eight host frames. Hosts drain exactly one `AUDIO_CHUNK_SAMPLES` chunk per
rendered frame, so the queue is a jitter cushion, not a working buffer. A
second of queued audio was also a second of latency.

## Consequences

Live heap for a 16KB NROM went from **489,839 to 97,885 bytes** — same test,
same ROM, both sides. Total working set is ~171KB, which fits an ESP32-S3's
internal SRAM with room for a display driver.

Two new test files hold the line:

- `tests/representation_golden.rs` pins host-visible output — framebuffer
  bytes, mixed APU samples, state hash — so a representation change that alters
  behavior fails loudly. Every change in this ADR passed it unmodified.
- `tests/memory_footprint.rs` measures live heap with a counting allocator and
  asserts both a ceiling and that the working set does not grow over a long
  run. Accounting is per-thread; a process-global counter would attribute other
  concurrently running tests' allocations to whichever measurement was in
  flight.

### Not yet done

**Memory.** Two reductions remain, both more invasive:

- `Cpu.memory: [u8; 0x10000]` (65KB) should become a real bus dispatch over
  2KB of work RAM. This is also the largest remaining throughput win — ADR 0003
  documents eight `write_byte` calls per instruction spent syncing the PPU
  register image into that array. It touches the Verus specs, so it is its own
  change. `CpuSnapshot.work_ram` is already `[u8; 2048]`, so the save-state
  format survives.
- Mappers own `prg_rom`/`chr_data` as `Vec<u8>`. ESP32 flash is memory-mapped,
  so borrowing `&'static [u8]` costs nothing and is what makes a 512KB MMC5
  cartridge possible at all. This threads a lifetime through `LoadedMapper`,
  `NesCore` and `CoreSnapshot`.

**Throughput.** Nothing here addresses the 72-cycles-per-dot figure. The
per-dot loop is the whole problem, and the candidates are, in order:
`pump_mapper_chr_fetches` running every dot when only MMC2/MMC4 record
anything; the 14-variant mapper enum dispatch per dot when only MMC3/MMC5 need
it; `update_sprite_zero_hit` running per visible pixel; and batching provably
inert dot ranges (the 22 VBlank scanlines are 7,502 dots that pay full dispatch
to do nearly nothing).

That last one is where the real multiple is, and it is also where determinism
is at risk. A scanline-batched PPU that diverges from the hosted build breaks
`state_hash` parity and the ROM credibility gates. Batching must be restricted
to dot ranges that are *provably* inert — rendering disabled, no mapper IRQ
armed, no pending register write — so the fast path stays bit-identical and the
proofs keep meaning something. A `--features fast-ppu` escape hatch excluded
from the gates would quietly fork the emulator and is explicitly rejected.

**Host crate.** An `nes-esp32` adapter would follow `nes-web`'s shape: a thin
wrapper over `NesCore` using the non-allocating `fill_*` accessors, `esp-hal`
for `no_std`, ROM borrowed from memory-mapped flash via `include_bytes!`, and
I2S for audio.

### Chip selection

ESP32-P4 (400MHz dual-core RISC-V, 768KB SRAM, upstream `rustc` target) is the
recommended target. ESP32-S3 (240MHz dual Xtensa, 512KB SRAM) is viable on
memory after this change but has 2.7x less cycle budget and requires the
`esp-rs` toolchain fork. ESP32-C3/C6 are upstream-supported but single-core at
160MHz, which the throughput measurement rules out.
