# NES Emulator (Proof-Forward + MCP-First)

REQUIRES FEATURE NOVA

This repository hosts a Rust NES emulator workspace focused on systems learning, deterministic behavior, and strong correctness checks.

## Workspace

- `crates/nes-core`: deterministic emulation core, command/query API, and stable TAS movie/recorder primitives.
- `crates/nes-mcp`: MCP tool surface mapped to core APIs.
- `crates/nes-desktop`: desktop input bridge and runtime adapter.
- `crates/nes-tui`: terminal (`ratatui`) runtime adapter.
- `crates/nes-web`: browser input bridge and runtime adapter.
- `crates/nes-proof`: Verus proof specs and lemmas.
- `crates/nes-test-harness`: replay + ROM gate tests.
- `crates/nes-netplay`: rollback netcode engine + relay protocol types.
- `crates/nes-relay`: room relay server for internet netplay sessions.

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
powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/verus-check.ps1
```

Coverage locally (same format uploaded in CI):

```powershell
cargo llvm-cov --workspace --all-features --all-targets --lcov --output-path lcov.info
```

## Runtime And ROM Config

Runtime and ROM paths are configured through `nes.toml` at the workspace root.
Netplay settings are configured in `[netplay]` (see `nes.example.toml`).

Desktop/TUI launch commands:

```powershell
cargo run -p nes-desktop --release -- ./roms/homebrew/homebrew.nes
cargo run -p nes-desktop --release -- --config ./nes.toml
cargo run -p nes-tui -- --config ./nes.toml
```

RTA mode (speedrunner-focused):

```powershell
# Strict RTA mode (auto-select profile by ROM hash)
cargo run -p nes-desktop --release -- --rta --rta-profiles-dir ./config/rta/profiles ./roms/homebrew/homebrew.nes

# Pre-start manual profile override
cargo run -p nes-desktop --release -- --rta --rta-profile smb-any --rta-profiles-dir ./config/rta/profiles ./roms/homebrew/homebrew.nes

# Calibration mode -> writes draft profile/report
cargo run -p nes-desktop --release -- --rta --rta-calibrate --rta-profile smb-any --rta-profiles-dir ./config/rta/profiles ./roms/homebrew/homebrew.nes
```

RTA profile + artifacts:
- Profiles live in `config/rta/profiles/*.toml` (see `config/rta/profiles/smb-any.example.toml`).
- Finished runs always write `*.run.json` to `runs/rta` (override with `--rta-runs-dir`).
- Per-frame input logs are optional (`[logging].save_input_log = true`).
- Calibration mode writes `<id>.draft.toml` and `<id>.draft_report.json`; draft profiles are not used by strict auto-selection.
- Hotkeys: `F9` manual split, `F10` finish calibration run.

Rollback netplay (across-town) flow:

```powershell
# Terminal 1: relay server
cargo run -p nes-relay -- --bind 0.0.0.0:4545

# Terminal 2: player 1
cargo run -p nes-desktop --release -- --netplay --netplay-relay <relay-host>:4545 --netplay-room river-city --netplay-player 1 ./roms/homebrew/homebrew.nes

# Terminal 3: player 2
cargo run -p nes-desktop --release -- --netplay --netplay-relay <relay-host>:4545 --netplay-room river-city --netplay-player 2 ./roms/homebrew/homebrew.nes
```

Relay can inject controlled network faults for rollback testing:

```powershell
cargo run -p nes-relay --release -- --bind 0.0.0.0:4545 --latency-ms 45 --jitter-ms 12 --loss-pct 0 --reorder-pct 25
```

Desktop netplay metrics now include `net_rtt_ms`, `net_jitter_ms`, `net_rollbacks`, `net_max_rb`, `net_desyncs`, and adaptive `net_delay_frames`.

WebAssembly build:

```powershell
cargo build -p nes-web --target wasm32-unknown-unknown
```

Web demo build + local serve (Trunk):

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/run_web_demo.ps1 -OpenBrowser
```

Web host ROM persistence:
- Uploaded ROMs are stored locally via IndexedDB for next launch.
- The last saved ROM auto-restores on startup.
- `Forget Saved ROM` clears the locally stored ROM bytes.

WASM path (web host -> core):

1. `web/index.html` declares the Rust artifact input (`../crates/nes-web/Cargo.toml`) for Trunk.
2. `crates/nes-web/Trunk.toml` sets the Trunk target to `../../web/index.html` and output to `../../web/dist`.
3. `web/app.js` imports `NesWebEmulator` from generated wasm glue and drives the browser loop.
4. `crates/nes-web/src/lib.rs` (`wasm-bindgen`) forwards JS calls to `WebRuntime`.
5. `crates/nes-web/src/runtime.rs` translates those calls into `nes_core::Command` execution and query reads.
6. `crates/nes-web/src/bridge.rs` maps DOM key codes to core button commands.

Desktop can optionally host MCP on the same live `NesCore` instance:

```powershell
cargo run -p nes-desktop --features mcp-host -- --mcp-host --mcp-bind 127.0.0.1:6502
```

Desktop quicksave / quickload:
- `F5` writes a manual save state for the current ROM.
- `F8` loads that manual save state back.
- Save files live under `./savestates/<rom-stem>-<hash8>.state.json`.
- Manual save/load is blocked while netplay rollback is active.

Automation scripts:

```powershell
# Deterministic demo sequence + one capture
powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/mcp_play_demo.ps1

# Hybrid controller: macro segments + deterministic micro-control + savestate retries
powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/mcp_hybrid_autoplay.ps1
```

For in-process automation, `nes_core::tas` is now the stable foundation when `nes-core` is built with `--features tas`: it records run-length encoded per-frame controller movies, replays them deterministically against `NesCore`, and can export the legacy macro script format for existing MCP tooling. That gives future search/planning work, including a possible `nes-ai` crate, a structured input tape instead of a stringly experiment.

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

Run rollback reliability soak gate (deterministic 2-peer fault simulation):

```powershell
cargo test -p nes-test-harness --test netplay_rollback -- --nocapture
```

Run only the bbbradsmith audio checks:

```powershell
cargo test -p nes-test-harness --test rom_bbbradsmith_audio -- --ignored --nocapture
```

Generate/update golden PCM captures for supported mapper ROMs (0/1/2/4):

```powershell
cargo run -p nes-test-harness --bin bbbradsmith_golden_capture -- --config ./nes.toml
```

Force overwrite existing golden files:

```powershell
cargo run -p nes-test-harness --bin bbbradsmith_golden_capture -- --config ./nes.toml --force
```

## Homebrew ROM

Build the in-repo custom ROM (no external assembler required):

```powershell
cargo run -p nes-test-harness --bin build_homebrew_rom
```

Run it:

```powershell
cargo run -p nes-desktop --release -- ./roms/homebrew/homebrew.nes
```

Or build and run in one command:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/run_homebrew.ps1
```
