# NES Emulator (Proof-Forward + MCP-First)

This repository hosts a Rust NES emulator workspace focused on systems learning, deterministic behavior, and strong correctness checks.

## Workspace

- `crates/nes-core`: deterministic emulation core and command/query API.
- `crates/nes-mcp`: MCP tool surface mapped to core APIs.
- `crates/nes-desktop`: desktop input bridge and runtime adapter.
- `crates/nes-tui`: terminal (`ratatui`) runtime adapter.
- `crates/nes-web`: browser input bridge and runtime adapter.
- `crates/nes-proof`: Verus proof specs and lemmas.
- `crates/nes-test-harness`: replay + ROM gate tests.

## v0 Quality Gates

`v0` is considered complete only when all are true:
- Playable behavior on desktop and web.
- ROM credibility targets pass (nestest + blargg suites as configured).
- MCP/core parity tests pass.
- Required Verus proofs pass.

## Verification Commands

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\verus-check.ps1
```

## Runtime And ROM Config

Runtime and ROM paths are configured through `nes.toml` at the workspace root.

Desktop/TUI launch commands:

```powershell
cargo run -p nes-desktop --release -- "C:\Users\markm\roms\Super Mario Bros. (World).nes"
cargo run -p nes-desktop --release -- --config .\nes.toml
cargo run -p nes-tui -- --config .\nes.toml
```

Desktop can optionally host MCP on the same live `NesCore` instance:

```powershell
cargo run -p nes-desktop --features mcp-host -- --mcp-host --mcp-bind 127.0.0.1:6502
```

Automation scripts:

```powershell
# Deterministic demo sequence + one capture
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\mcp_play_demo.ps1

# Hybrid controller: macro segments + deterministic micro-control + savestate retries
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\mcp_hybrid_autoplay.ps1
```

ROM harness tests read from:
- `roms.smb`
- `roms.nestest`
- `roms.blargg_cpu`
- `roms.bbbradsmith_audio_suite_dir` (directory containing the bbbradsmith `.nes` suite)
- `roms.bbbradsmith_audio_golden_dir` (directory containing `.s16le.pcm` reference captures)

ROM tests remain ignored by default until those files are present locally/CI.

Run ROM credibility tests (including ignored ROM suites) with:

```powershell
cargo test -p nes-test-harness -- --ignored
```

Run only the bbbradsmith audio checks:

```powershell
cargo test -p nes-test-harness --test rom_bbbradsmith_audio -- --ignored --nocapture
```

Generate/update golden PCM captures for supported mapper ROMs (0/1/2):

```powershell
cargo run -p nes-test-harness --bin bbbradsmith_golden_capture -- --config .\nes.toml
```

Force overwrite existing golden files:

```powershell
cargo run -p nes-test-harness --bin bbbradsmith_golden_capture -- --config .\nes.toml --force
```
