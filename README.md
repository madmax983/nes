# NES Emulator (Proof-Forward + MCP-First)

This repository hosts a Rust NES emulator workspace focused on systems learning, deterministic behavior, and strong correctness checks.

## Workspace

- `crates/nes-core`: deterministic emulation core and command/query API.
- `crates/nes-mcp`: MCP tool surface mapped to core APIs.
- `crates/nes-desktop`: desktop input bridge and runtime adapter.
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

## ROM Harness Environment Variables

- `NESTEST_ROM_PATH`
- `BLARGG_CPU_ROM_PATH`

ROM tests are currently checked in as ignored gates until ROM paths are supplied in CI or local runs.
