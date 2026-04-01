# 🔭 Vantage: Spec for Auto-Pause on Window Focus Loss

## 👤 User Story
"As a Player, I want the emulator to automatically pause gameplay when I switch to another window or application, so that I do not miss anything or die in the game while I am looking away."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Currently, when a user alt-tabs or clicks on a second monitor (e.g., to check Discord, read a walkthrough, or answer a message), the emulator continues to run in the background. In games without a built-in pause button, or if the user forgets to manually trigger the emulator's overlay, the game state advances out of their control. This leads to frustrating deaths and forces the user to reload a savestate, degrading the core desktop experience. Auto-pause reduces this friction and matches modern PC gaming standards.

## 📈 Success Metrics
- **Reliability:** 100% of window focus loss events successfully trigger the pause state.
- **Resumption:** Returning focus to the window instantly resumes gameplay without dropped inputs.
- **Engagement:** Decrease in user reports complaining about "game running in background."

## 🕵️ Gap Analysis
- **Market View:** Standard feature in nearly every modern desktop application and emulator (often a toggleable setting).
- **Our Gap:** We only pause when the user explicitly opens the in-game overlay menu via `Escape` or selects it from the native menu bar. We do not listen to OS-level window focus events to halt the execution loop.

## ✅ Acceptance Criteria
- Must detect when the main emulator window loses OS focus.
- Must pause emulation execution when focus is lost.
- Must present a visual indicator (e.g., a "Paused" text overlay or slightly dimmed screen) so the user knows the state is frozen.
- Must automatically resume emulation execution when the main emulator window regains focus.
- Must provide an option in `nes.toml` (`[ui] auto_pause_on_focus_loss = true/false`) to disable this behavior for users who want background execution (e.g., during netplay waiting or listening to audio).
- Must *not* auto-resume if the emulator was already manually paused (via the overlay menu) prior to losing focus.

## 🚫 Out of Scope
- Muting audio in the background without pausing the game (Phase 2 setting).
- Auto-pausing specifically for Netplay (Netplay handles its own sync; auto-pause may need to be strictly disabled during active netplay to prevent rollback storms).
