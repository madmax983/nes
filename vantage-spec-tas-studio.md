# 🔭 Vantage: Spec for TAS Studio

## 👤 User Story
"As a Tool-Assisted Speedrunner (TASer) or automation engineer, I want an interactive 'Piano Roll' style TAS Studio, so that I can visually edit controller inputs frame-by-frame, test new strats, and immediately see the results deterministically without manually editing JSON files."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
We currently have a solid, deterministic core and the `nes_core::tas` API for generating and replaying movies. However, creating or editing a TAS is currently an offline, manual process (editing JSON files) or requires programmatic automation via scripts. A built-in, visual "Piano Roll" editor (TAS Studio) removes the friction of building optimal runs. This empowers the speedrunning community to find new tricks within our emulator environment and drives adoption of our platform as the primary tool for modern TAS development.

## 📊 Success Metrics
- **Performance:** Rendering the piano roll overlay (even for movies with 10,000+ frames) maintains 60fps interaction speed.
- **Utility:** A user can load a movie, manually toggle the "A" button input on a specific frame within the UI, and step the emulator to see the immediate effect.
- **Adoption:** 20% of users loading external TAS `.tas.json` files engage with the TAS Studio UI to modify inputs.

## 🕵️ Gap Analysis
- **Market View:** Legacy emulators (like FCEUX, BizHawk) have established "TAS Studio" modes featuring a piano roll interface, allowing users to scroll through frames, toggle buttons, and use savestates to "branch" their movie recordings.
- **Our Gap:** We provide the underlying engine to run TAS movies perfectly, but we lack the UI to author or edit them visually. Users must write external scripts or manually edit JSON files, lacking real-time visual feedback on how input changes affect the game state.

## ✅ Acceptance Criteria
- Must provide a "Piano Roll" UI overlay (accessible in `nes-desktop` or `nes-tui`).
- Must display a scrolling list of frames, showing the controller 1 (and optionally controller 2) inputs for each frame (A, B, Select, Start, Up, Down, Left, Right).
- Must allow clicking/toggling individual inputs on a specific frame.
- Must visually indicate the "current execution frame" as the emulator steps forward.
- Must integrate with the existing `nes_core::tas` module to load and save edited movies.
- Must allow jumping/seeking to a specific frame (likely involving internal rewinding/savestate usage, or at least restarting from a known snapshot).

## 🚫 Out of Scope
- Advanced macro generation or auto-solving (e.g., A* search for inputs) within the UI.
- Direct integration with the `nes-ai` reinforcement learning loops.
- Support for 4-player multiplayer adapters.
