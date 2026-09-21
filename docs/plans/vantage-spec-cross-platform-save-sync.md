# Spec: Cross-Platform Save Sync

As a Multi-Device Player, I want my battery saves and save states to automatically sync across devices, so that I can start a game on my desktop and continue playing on the web emulator seamlessly.

**So What?**
Players today expect mobility. Being locked to a single device reduces engagement. By providing a secure, transparent save-sync mechanism, we bridge the gap between `nes-desktop` (for high fidelity/recording) and `nes-web` (for casual access), significantly increasing overall session frequency.

**Metric Definition**
- Success = >90% of save sync operations complete in under 500ms without user intervention.
- Less than 1% conflict rate requiring manual user resolution.

## Gap Analysis
Most generic emulators rely on manual file transfers or complex external cloud syncing configurations which break easily across web vs. native platforms. Our solution must be zero-config (or near-zero), using a simple auth token or relying on our existing relay server infrastructure.

## Acceptance Criteria
- Must synchronize battery-backed saves and manual save states.
- Must provide a seamless bridge between `nes-desktop` and `nes-web`.
- Must handle conflict resolution gracefully (e.g., prompting the user if a divergent save timeline exists).
- Must fail-open: if offline, the emulator falls back to the local save transparently.

## Out of Scope
- Syncing large video recordings or AI policy checkpoints.
- Real-time multiplayer synchronization (this is handled by our Rollback Netplay feature).
