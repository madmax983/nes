**[Refactoring redundant state reset logic in nes-desktop/src/main.rs]
**Learning:** Found a 13-line block of logic repeated identically across `execute_app_action` for the `AppAction::OpenRom`, `AppAction::LoadSlot`, and `AppAction::Reset` branches. The block cleared the audio buffer, reset the rewind state, recorded a new base frame for the time machine, and reset performance metrics.
**Action:** Extracted this state-resetting block into a private helper function `reset_core_metrics_and_time_machine(ctx: &mut AppContext<'_>)` and replaced the duplicate blocks with a single call, eliminating ~25 lines of duplicate boilerplate.

**[Refactoring redundant state reset logic in nes-desktop/src/main.rs]
**Learning:** Found a 13-line block of logic repeated identically across `execute_app_action` for the `AppAction::OpenRom`, `AppAction::LoadSlot`, and `AppAction::Reset` branches. The block cleared the audio buffer, reset the rewind state, recorded a new base frame for the time machine, and reset performance metrics.
**Action:** Extracted this state-resetting block into a private helper function `reset_core_metrics_and_time_machine(ctx: &mut AppContext<'_>)` and replaced the duplicate blocks with a single call, eliminating ~25 lines of duplicate boilerplate.
