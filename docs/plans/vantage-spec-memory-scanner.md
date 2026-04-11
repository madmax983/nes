# Spec: Memory Scanner (Cheat Discovery)

## 👤 User Story
As a Power User or Game Modder, I want to scan and filter the NES RAM in real-time while playing, so that I can discover new memory addresses to use for custom cheat codes (like infinite health or extra lives) for undocumented games.

## 💼 Business Problem
While standard cheat codes exist for popular titles, many homebrew games, romhacks, and obscure titles lack cheat databases. Retaining power users and the romhacking community requires advanced tools. By providing a built-in memory scanner, we transform the emulator from a simple player into a developer/modder platform, increasing engagement from high-value technical users who often contribute back to the ecosystem.

**Success Metric**:
- A user can narrow down a specific memory address (e.g., health value) within 5 scan iterations.
- Memory scans complete in < 50ms to prevent emulation stuttering.

## ✅ Acceptance Criteria
- Must provide a UI overlay in `nes-desktop` to initiate a "New Scan" for an exact value or unknown initial value.
- Must allow "Next Scan" filtering (e.g., "Decreased", "Increased", "Changed", "Unchanged", "Exact Value").
- Must display a list of memory addresses matching the current filter, updating in real-time.
- Must allow the user to easily convert a discovered memory address into an active cheat code.
- Must gracefully handle scan state resets and provide clear feedback on how many addresses remain.

## 🚫 Out of Scope
- Support for complex multi-level pointer scanning (NES games rarely use deep pointer chains).
- Full memory hex editor views (the focus is purely on targeted search and filtering).
