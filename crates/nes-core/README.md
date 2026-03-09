# nes-core

`nes-core` is the deterministic emulation core shared by desktop, TUI, web, and MCP hosts.

## Scope

- 6502-compatible CPU execution, including common unofficial opcodes used by commercial games.
- Dot-stepped PPU with framebuffer output (`256x240 RGBA`).
- APU channel simulation (pulse/triangle/noise/DMC) with mixed PCM output.
- Mapper-backed PRG/CHR banking (currently NROM, MMC1, UxROM, CNROM, MMC3, AxROM, GxROM).
- Save/load state snapshots, command replay, and stable TAS movie/recorder primitives.

## Host Model

Hosts control emulation through `NesCore`:

1. Load ROM bytes with `load_ines_rom`.
2. Drive progression with `execute(Command::StepCpu|StepScanline|StepFrame)`.
3. Push controller state with `SetControllerState` or button press/release commands.
4. Pull video/audio (`fill_framebuffer_rgba`, `audio_chunk_i16`).

Higher-level automation can stay in-process through `nes_core::tas` when the `tas` feature is enabled:
- `TasRecorder` captures per-frame controller state into a deterministic movie.
- `TasMovie::replay` applies that movie back onto a `NesCore`.
- `TasMovie::to_macro_script` exports the legacy `nes-mcp` line format when player 2 is unused.

The core remains platform-agnostic. Windowing, audio devices, web bindings, and MCP transport are out-of-scope for this crate.

## Timing Model

- CPU advances in instruction steps, each returning cycle count.
- Each CPU cycle clocks:
  - APU once
  - PPU three times
- DMA and interrupt service routines consume additional cycles to preserve ordering invariants.

This preserves deterministic progression for replay, testing, and state hashing.

## Memory and Mapping

- CPU visible RAM/register behavior is synchronized through explicit bus write/read application.
- PPU register mirrors are normalized to `0x2000..=0x2007`.
- Cartridge PRG windows are sourced from mapper implementations and copied into CPU visible PRG space.

## ROM Support

- iNES and NES 2.0 headers are parsed.
- Supported mapper IDs: `0` (NROM), `1` (MMC1 subset), `2` (UxROM), `3` (CNROM), `4` (MMC3 subset), `7` (AxROM), `66` (GxROM).
- Unsupported configurations return structured `RomError` values.

## Determinism and Tooling

- `save_state`/`load_state` round-trip full machine state needed for deterministic restore.
- `state_hash` offers a compact change detector for regression tooling.
- `replay` applies command streams for reproducible host-independent runs.
- `tas` (feature-gated) adds a structured movie format that future tooling can mutate, branch, serialize, and replay.

## Limitations (Current)

- Mapper support is intentionally narrow and focused on early playability targets.
- PPU behavior targets practical game compatibility and test fidelity but is not a full transistor-level model.
- Audio path favors stability and host integration over cycle-perfect analog modeling.

## Testing Strategy

Integration and property tests in `tests/` cover:

- Bus map contracts
- CPU stepping and tracing behavior
- Mapper semantics
- PPU timing and register contracts
- DMA/IRQ sequencing
- APU fidelity and output contracts
- End-to-end command/query behavior

Run with:

```powershell
cargo test -p nes-core
```
