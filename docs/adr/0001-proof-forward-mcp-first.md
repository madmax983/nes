# ADR 0001: Proof-Forward + MCP-First NES Architecture

Date: 2026-02-21
Status: Accepted

## Context

The emulator must satisfy two outcomes for v0:
1. Playable desktop and web execution.
2. Credible ROM validation behavior.

The project also requires strict MCP parity for user-facing input/output behavior and formal verification for critical invariants.

## Decision

Adopt a proof-forward and MCP-first workspace architecture:
- `nes-core` is the single emulation state authority.
- `nes-mcp` maps user-facing tools directly to `nes-core` command/query APIs.
- `nes-desktop` and `nes-web` are thin adapters with no direct state mutation outside `nes-core`.
- `nes-proof` contains Verus proofs for CPU state legality, bus mapping contracts, and mapper bank safety.
- `nes-test-harness` contains deterministic replay and ROM-gated tests.

## Consequences

Positive:
- Prevents frontend/MCP behavior drift.
- Adds formal guardrails around invalid emulator states.
- Improves deterministic debugging with command replay.

Tradeoffs:
- Early implementation speed is lower due to proof authoring and parity tests.
- CI is heavier (fmt, clippy, tests, Verus).

## Guardrails

No user-facing behavior is accepted without:
- an MCP tool mapping,
- a runtime test,
- and proof coverage for critical invariant classes.
