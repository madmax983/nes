# 🔭 Vantage: Spec for TAS Piano Roll Editor

## 👤 User Story
"As a Tool-Assisted Speedrunner (TASer), I want a visual piano-roll editor for controller inputs, so that I can precisely modify individual frames, splice segments, and visualize my input sequence without editing raw text files."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
We have robust, deterministic TAS recording primitives that generate artifacts used by our AI pipeline and tests. However, human creators must manually edit text files or use fragile macro scripts to build these runs. By adding a visual piano-roll editor, we significantly lower the barrier to entry for TAS creation, bridging the gap between raw execution capability and human creativity. This empowers the community to build content that showcases the emulator's precision.

## 📊 Success Metrics
- **Performance:** Rendering the piano roll for a 1-hour TAS (216,000 frames) takes less than 100ms.
- **Utility:** A user can insert a single A-button press at frame 14,320 and immediately replay the segment.
- **Adoption:** 80% of TAS artifacts generated through our platform are modified using the editor.

## 🕵️ Gap Analysis
- **Market View:** Established emulators like FCEUX have dedicated TAS editors with piano rolls, frame counters, and rerecord tracking.
- **Our Gap:** We provide the underlying recording/playback mechanisms but no interface for editing the recorded tape.

## ✅ Acceptance Criteria
- Must visualize inputs (A, B, Select, Start, Up, Down, Left, Right) over time in a grid format.
- Must allow toggling individual inputs on/off for any specific frame.
- Must support copy/pasting blocks of inputs.
- Must highlight the "current" frame being executed by the emulator in real-time.
- Must safely parse and resave the underlying TAS format without data loss.

## 🚫 Out of Scope
- Advanced branching or "Movie Tree" visualizers (Phase 2).
- Automated input generation / brute-forcing within the editor UI.
