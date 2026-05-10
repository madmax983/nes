# 🔭 Vantage: Spec for TAS Studio Editor

## 👤 User Story
"As a TAS Creator, I want a piano-roll style input editor and frame-stepping environment, so that I can visually create, edit, and optimize tool-assisted speedruns frame-by-frame without writing JSON by hand."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our emulator has a robust deterministic core (`nes_core::tas`) capable of recording and replaying runs perfectly. However, the authoring experience requires either external script generation or manual `.tas.json` editing. By providing a built-in TAS Studio Editor with a visual piano-roll for inputs, we lower the barrier to entry for TAS creators and position our emulator as a modern alternative to legacy tools (like FCEUX or BizHawk). This drives adoption within the speedrunning and TAS community and leverages our existing deterministic engine.

## 📊 Success Metrics
- **Performance:** Rendering the TAS piano-roll overlay and editing inputs adds zero measurable overhead to the core emulator loop when unpaused.
- **Utility:** A user can load a `.tas.json` file, visually modify input on a specific frame, and seamlessly resume playback from a savestate exactly on that frame.
- **Adoption:** 20% of users loading a ROM interact with the TAS Studio editor within their first 10 sessions.

## 🕵️ Gap Analysis
- **Market View:** Existing TAS tools like BizHawk's TAStudio provide extensive frame-by-frame input grids, savestate integration, and branch tracking.
- **Our Gap:** We have the perfect backend (`nes_core::tas` and deterministic core) but absolutely zero UI frontend for input authoring. Users are forced to drop to the command line or use external JSON editors, which breaks the workflow and prevents interactive optimization.

## ✅ Acceptance Criteria
- Must provide a piano-roll/grid UI overlay (via `nes-desktop` or `nes-tui`) displaying frames as rows and buttons as columns.
- Must allow users to click to toggle specific button inputs (A, B, Select, Start, Up, Down, Left, Right) on any given frame.
- Must integrate with savestates so that playback can instantly rewind to the frame being edited.
- Must be able to load, edit, and save changes back out to the `nes_core::tas` JSON format.
- Must allow playback controls (Play, Pause, Step Forward Frame, Step Backward Frame) directly from the editor view.

## 🚫 Out of Scope
- TAS multi-branching or "timeline tree" management (Phase 2).
- Automatic TAS optimization algorithms or brute-force search bots (Phase 2).
- Visualizing PPU rendering ahead of time for future frames.
