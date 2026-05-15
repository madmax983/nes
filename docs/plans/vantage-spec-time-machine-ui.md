# 🔭 Vantage: Spec for Time Machine UI

## 👤 User Story
"As a Player, I want to hold a hotkey to smoothly rewind gameplay in real-time, so that I can undo a mistake or a missed jump without having to reload a full manual savestate."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Classic NES games are notoriously difficult and unforgiving. While we offer manual F5/F8 savestates, forcing users to proactively manage state ruins the flow of gameplay and breaks immersion. A seamless "Rewind" feature acts as a modern safety net, significantly reducing frustration and lowering the barrier to entry for modern players. This drastically improves user retention and session length, matching the Quality of Life standards set by popular mainstream retro collections (e.g., Nintendo Switch Online, RetroArch).

## 📊 Success Metrics
- **Performance:** Recording historical state in the background must add < 1ms overhead per frame during normal gameplay.
- **Responsiveness:** Rewinding begins within 1 frame of the hotkey being pressed.
- **Adoption:** 60% of players utilize the rewind hotkey at least once during a >15-minute session.

## 🕵️ Gap Analysis
- **Market View:** Top-tier consumer emulators and commercial retro ports consider real-time rewind a mandatory, baseline feature.
- **Our Gap:** We have recently built the underlying compression and timeline logic (`crates/nes-rewind`), but it is purely an engine. The `nes-desktop` client currently lacks any UI integration, hotkey binding, or visual feedback to actually expose this Time Machine to the end user.

## ✅ Acceptance Criteria
- Must bind a default hotkey (e.g., `Backspace` or `Left Trigger`) to initiate the rewind action in `nes-desktop`.
- Must smoothly reverse gameplay state frame-by-frame (or via compressed deltas) while the hotkey is held down.
- Must immediately resume normal forward play when the hotkey is released.
- Must provide clear visual feedback while rewinding (e.g., an on-screen "⏪ Rewind" icon or a screen tint).
- Must automatically disable or ignore the rewind hotkey during active Netplay sessions to prevent irrecoverable desyncs.

## 🚫 Out of Scope
- A visual timeline scrubber UI allowing users to jump to specific points (this is Phase 2, Phase 1 is just the continuous hold-to-rewind).
- Rewinding through manual savestate loads (rewind history is cleared upon loading a state).
