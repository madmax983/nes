# Spec: Cheat Code Support (Game Genie)

## 👤 User Story
As a Casual Gamer or Speedrunner practicing specific sections, I want to input 6-character or 8-character NES cheat codes (Game Genie), so that I can modify game state like infinite lives or starting levels without manually editing RAM.

## 💼 Business Problem
NES emulation has a steep learning curve. Players drop off when games are too difficult or when they have to repeat tedious sections. Providing built-in, easy-to-use cheat code support directly in the emulator UI increases user retention, reduces frustration, and expands our TAM (Total Addressable Market) to casual players who just want to "experience" the games.

**Success Metric**:
- Cheat codes can be added and removed via the desktop overlay menu.
- Parsing latency for cheat codes is < 1ms.

## ✅ Acceptance Criteria
- Must support standard 6-character and 8-character Game Genie codes (e.g., `GOSSIP`, `ZEXPYGLA`).
- Must handle invalid codes gracefully without panicking, presenting a clear error message.
- Must provide an in-game overlay UI in `nes-desktop` to add, toggle, and remove codes.
- Must persist added cheat codes across emulation sessions for the same ROM (or at least during the active session).
- Must apply memory patches dynamically without permanently altering the source ROM file.

## 🚫 Out of Scope
- Automatic downloading of cheat code databases from the internet.
- Support for complex multi-line cheat scripts or Action Replay.
- Real-time memory scanning to *discover* new cheat codes (this is a separate "Memory Scanner" feature).
