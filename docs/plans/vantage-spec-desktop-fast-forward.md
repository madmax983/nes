# 🔭 Vantage: Spec for Desktop Fast Forward

## 👤 User Story
As a Desktop Player, I want to hold a hotkey to fast-forward gameplay, so that I can quickly skip through unskippable cutscenes, long RPG battles, or tedious overworld travel.

## 💼 Business Problem (So What?)
Modern players have less free time and lower tolerance for the slow pacing of retro games. A lack of fast-forward functionality leads to user frustration and drop-off, particularly in RPGs or games with slow transitions. Providing a fast-forward feature improves the perceived quality of life (QoL) and keeps our emulator competitive with industry standards (like RetroArch), retaining users who would otherwise abandon slow-paced games.

## 📈 Success Metrics
- Fast-forward successfully accelerates emulation speed by at least 2x when triggered.
- Audio remains stable (or is gracefully muted/resampled) during fast-forward.
- Zero crashes when toggling fast-forward repeatedly.

## 🕵️ Gap Analysis
- **Market View:** Fast-forward is a standard, expected feature in all modern retro emulators.
- **Our Gap:** The `nes-web` crate has a `tick_budget_permille` to allow fast-forwarding, but `nes-desktop` has no input binding or runtime support for accelerating the execution loop.

## ✅ Acceptance Criteria
- Must introduce a default hotkey (e.g., `Tab` or `Space`) to trigger fast-forward mode.
- Must execute the emulator loop at an accelerated rate (e.g., 2x or configurable) while the hotkey is held.
- Must return to normal execution speed immediately when the hotkey is released.
- Must not cause the audio engine to panic (acceptable to drop frames or mute audio while fast-forwarding if resampling is too complex for v1).
- Must disable fast-forward automatically or ignore the hotkey during active Netplay sessions to prevent desyncs.

## 🚫 Out of Scope
- Rewind functionality (tracked separately).
- Variable speed sliders (e.g., smoothly sliding between 1.1x and 3.0x speed).
