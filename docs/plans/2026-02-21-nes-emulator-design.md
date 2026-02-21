# NES Emulator Design (Proof-Forward + MCP-First)

Date: 2026-02-21
Status: Approved

## Goal

Build a Rust NES emulator for systems learning and fun that is both:
- Playable (desktop and web)
- Credible on standard ROM test suites

The design is proof-forward: critical invariants are specified and proven early with Verus, then implemented in runtime code and validated with tests.

## Locked Decisions

- Full stack scope: CPU + PPU + APU + input + mapper support.
- v0 success gate: both playable and test-ROM credible.
- Timing model: cycle-stepped core.
- Frontend stack: `winit + pixels + cpal` on desktop.
- Platform target: desktop + web from the beginning.
- Mapper scope for v0: NROM + UxROM + MMC1.
- Repository topology: workspace with shared core and thin platform adapters.
- Verification posture: proof-forward (not test-only).
- MCP requirement: every user-facing input/output has a corresponding MCP tool.
- MCP payload mode: hybrid (metadata stream + optional chunk fetch).

## Workspace Architecture

Planned crates:
- `crates/nes-core`: deterministic emulation core (CPU, PPU, APU, bus, mappers, scheduler).
- `crates/nes-proof`: Verus specs and proofs for critical invariants and transition rules.
- `crates/nes-mcp`: MCP server/tool surface mapped directly to core command/query APIs.
- `crates/nes-desktop`: desktop adapter (window/input/video/audio) using `winit/pixels/cpal`.
- `crates/nes-web`: browser adapter (`wasm-bindgen/web-sys`) for canvas/audio/input.
- `crates/nes-test-harness`: ROM runner, deterministic replay, parity and regression tests.

Core rule: frontends and MCP call the same `nes-core` command/query layer. No frontend-specific mutation path.

## Core Execution Model

`nes-core` exposes cycle stepping primitives orchestrated by a scheduler:
- `step_cpu_cycle`
- `step_ppu_cycle`
- `step_apu_cycle`
- higher-level wrappers such as `step_frame` built from cycle semantics

This ensures one timing truth across desktop and web.

## MCP-First I/O Contract

Every user-facing behavior has an MCP equivalent.

Input/control tools:
- `load_rom`
- `set_controller_state`
- `press_button`
- `release_button`
- `reset`
- `power_cycle`
- `pause`
- `resume`
- `set_speed`

Output/inspection tools:
- `get_frame`
- `get_audio_chunk`
- `get_fps`
- `get_emulator_state`
- `read_memory`
- `read_registers`
- `disassemble_at`

Debug/state tools:
- `step_cpu`
- `step_scanline`
- `step_frame`
- `set_breakpoint`
- `clear_breakpoint`
- `save_state`
- `load_state`

Payload strategy:
- stream metadata (frame/audio availability, dimensions, timing, sequence IDs)
- fetch frame/audio chunks on demand

## Formal Verification Scope (v0)

Priority proofs in `nes-proof`:
- CPU state legality (flags/register invariants preserved).
- Decode/execute contract for verified opcode subsets.
- Bus resolution safety (legal and unambiguous memory region mapping).
- Mapper bank safety for NROM/UxROM/MMC1.
- Scheduler/timing invariants (monotonic counters, legal ordering).

Proofs define constraints first; runtime implementation follows the proved shape.

## Test Strategy

Tests complement proofs (proofs prevent forbidden states; tests prevent forgotten behavior):
- Unit tests for instruction behavior and subsystem APIs.
- Property tests for bus and mapper boundary behavior.
- ROM-based tests (`nestest`, blargg CPU/PPU/APU suites).
- Deterministic replay tests from command/event logs.
- MCP parity tests: tool call results must match direct core API results.

## Milestones

- M0: Workspace bootstrap, lint/test/proof CI skeleton, crate boundaries.
- M1: Verified CPU spine + minimal executable instruction path.
- M2: Verified bus/memory mapping + NROM/UxROM/MMC1 contracts.
- M3: Cycle scheduler invariants + runtime stepping.
- M4: Desktop/web adapters on shared core APIs.
- M5: MCP tool parity + replay harness.
- M6: Playable target + ROM suite credibility gate.

## Error Model

- `nes-core`: typed domain errors (ROM validity, mapper config, illegal transition, bounds violations).
- `nes-mcp`: stable tool error mapping for automation-friendly client behavior.
- Frontends: user-friendly presentation only; no hidden state mutation or recovery logic.

## Definition of Done for v0

A dated CI run must show all of the following:
- Playable game target succeeds on desktop and web.
- Agreed ROM suites meet pass threshold.
- MCP parity tests are green.
- Proof checks for required invariants are green.
