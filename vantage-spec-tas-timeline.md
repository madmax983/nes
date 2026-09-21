# 🔭 Vantage: Spec for TAS Timeline Editor

## 👤 User Story
"As a TAS (Tool-Assisted Speedrun) creator, I want a visual timeline editor overlay, so that I can scrub through and edit input frames interactively without manually modifying JSON files."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our emulator provides exceptional accuracy and stable TAS movie/recorder primitives, but modifying TAS runs requires manual editing of JSON files or running separate scripts. By providing a dedicated real-time TAS timeline editor, we lower the barrier to entry for speedrun creators and AI-assisted players. This makes our emulator a highly sought-after tool for creating optimized tool-assisted runs directly within the UI, reducing reliance on external editors.

## 📊 Success Metrics
- **Performance:** Editing inputs dynamically should cause no stuttering or lag.
- **Utility:** A user can seamlessly toggle inputs on any specific frame.
- **Adoption:** 30% of power users will use the TAS Timeline Editor for recording and optimizing runs.

## 🕵️ Gap Analysis
- **Market View:** Specialized TAS emulators (like FCEUX or BizHawk) have robust piano-roll style timeline editors and input viewers.
- **Our Gap:** We currently support TAS playback and recording primitives (e.g. `nes-ai` writes replayable TAS artifacts) but have no graphical timeline editor in `nes-desktop` or `nes-tui` to interact with this data easily.

## ✅ Acceptance Criteria
- Must provide a toggleable "Timeline Editor" overlay in the desktop client.
- Must display a piano-roll interface for all controller inputs (A, B, D-Pad, Select, Start) frame by frame.
- Must allow the user to click to toggle inputs on any given frame.
- Must allow scrubbing the emulator state forward and backward synchronously with the timeline cursor.
- Must support saving the modified input sequence back to a `.tas.json` artifact.

## 🚫 Out of Scope
- Automatic gameplay optimization (AI integration).
- Multi-track TAS merging.
- Exporting the timeline directly to video (Phase 2).
