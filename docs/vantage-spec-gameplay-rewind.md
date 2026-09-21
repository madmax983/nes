# Product Spec: Real-time Gameplay Rewind

## 👤 User Story
As a casual player and speedrunner, I want to seamlessly rewind gameplay in real-time, so that I can quickly undo mistakes and practice difficult segments without managing manual save states.

## 🏢 Business Problem ("So What?")
Retro games are notoriously unforgiving. Modern emulators (like RetroArch) have normalized the "Rewind" feature as a baseline expectation. Without it, our emulator is less attractive to casual audiences and speedrunners looking for a robust practice environment. Given we already have a robust rollback and snapshot system for Netplay, exposing a user-facing Rewind feature is a high-ROI leverage of existing architecture.

## 🎯 Metric Definition (Success)
- **Performance:** Rewinding must maintain a steady 60 FPS (processing latency < 16ms per frame reversed).
- **Resource Limits:** Ring buffer storing state history must not exceed 100MB of RAM for up to 60 seconds of rewind history.
- **Reliability:** State must remain 100% deterministic during and after a rewind sequence.

## 🔍 Gap Analysis
- **Current State:** Users must use F5/F8 to manually save and load states.
- **Competitors:** Mesen, RetroArch, and Nestopia all offer seamless hotkey-driven real-time rewind.
- **Our Advantage:** Our emulator is built on a deterministic core with Verus proofs and an existing rollback engine for netplay. We can provide a highly reliable rewind feature with minimal new architectural complexity.

## ✅ Acceptance Criteria
- A configurable hotkey (default: Backquote/Tilde or Left Trigger on gamepads) initiates Rewind when held.
- The game steps backward continuously while the hotkey is held, at a configurable speed (e.g., 1x, 2x, 4x).
- Game resumes normal forward execution seamlessly when the hotkey is released.
- A visual indicator (e.g., an icon or UI overlay) is displayed while rewinding.
- Rewind history length is configurable in `nes.toml` (e.g., 10, 30, 60 seconds).
- Rewind is disabled automatically during Netplay sessions to prevent desyncs.

## 🚫 Out of Scope
- Branching timelines / visual state trees (e.g., Braid-style timeline scrubbing UI).
- Video-based rewind (we are rewinding emulation state, not rendering video frames backwards).
- Saving rewind history to disk between sessions.
