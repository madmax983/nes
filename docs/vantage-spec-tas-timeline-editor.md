# 🔭 Vantage: Spec for Visual TAS Timeline Editor

## 👤 User Story
"As a TAS (Tool-Assisted Speedrun) Creator, I want a visual timeline editor (Piano Roll) for controller inputs, so that I can intuitively see, modify, and optimize inputs frame-by-frame without manually editing serialized data files."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our emulator has a robust deterministic core and stable TAS movie/recorder primitives (`nes_core::tas`). However, creators must edit serialized data files to refine their runs, which is tedious, error-prone, and visually disconnected from the gameplay. By introducing a visual "Piano Roll" editor, we drastically lower the barrier to entry for TAS creation, making our tool the preferred workstation for speedrun routing and optimization, capturing the active TAS community.

## 📊 Success Metrics
- **Utility:** A user can successfully toggle an input (e.g., the 'A' button) on a specific frame and see the emulator state reflect the change within 1 second.
- **Adoption:** 50% of users who utilize the `nes_core::tas` recording functionality also open the Timeline Editor during their session.
- **Efficiency:** 80% reduction in time taken to correct a 10-frame input sequence compared to manual file editing.

## 🕵️ Gap Analysis
- **Market View:** Specialized development emulators have robust debugging suites including piano rolls.
- **Our Gap:** We possess the technical primitives (stable TAS movie/recorder, snapshot-start, deterministic replay) but completely lack a user-facing UI to inspect or edit the generated input tape interactively.

## ✅ Acceptance Criteria
- Must provide a separate UI window or overlay tab displaying a scrolling, frame-by-frame timeline of inputs (A, B, Select, Start, Up, Down, Left, Right).
- Must allow clicking on the timeline grid to toggle specific inputs on or off for a given frame.
- Must support inserting or deleting empty frames to shift the subsequent timeline.
- Must synchronize the emulator's current execution frame with a visual playhead on the timeline.
- Must allow scrubbing the playhead backward and forward along the timeline.

## 🚫 Out of Scope
- Automated brute-force search UI / Lua scripting integrations (Phase 2).
- Multi-controller (Player 2+) timeline editing (Phase 1 focuses on Player 1).
