# 🔭 Vantage: Spec for TAS Piano Roll Editor

## 👤 User Story
"As a Tool-Assisted Speedrunner (TASer), I want a visual piano roll editor for controller inputs, so that I can easily view, modify, and fine-tune individual button presses on a per-frame basis to optimize my speedruns."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our emulator supports generating and replaying TAS artifacts (`nes-ai` and `nes_core::tas`), but editing these inputs requires manipulating raw JSON or text files. This is highly error-prone and tedious. By providing a built-in visual piano roll editor, we empower the speedrunning and automation community to iterate faster, lowering the barrier to entry for creating high-quality TAS runs. This turns our emulator into a complete end-to-end TAS authoring suite, driving adoption among power users.

## 📊 Success Metrics
- **Utility:** A user can successfully correct a missed 1-frame input (e.g., a late jump) entirely within the UI.
- **Adoption:** 60% of users loading or saving a `.tas.json` file open the piano roll editor during their session.

## 🕵️ Gap Analysis
- **Market View:** Specialized TAS emulators (like FCEUX or BizHawk) feature mature, grid-based piano roll editors that visualize inputs across frames, allowing drag-and-drop editing and easy visual debugging.
- **Our Gap:** We have the deterministic core and the `.tas.json` recording/playback infrastructure, but we completely lack a user interface for editing or visualizing the recorded input sequence frame-by-frame.

## ✅ Acceptance Criteria
- Must provide a separate UI window or overlay tab to visualize the loaded TAS input sequence.
- Must display a scrolling grid (piano roll) where rows represent frames and columns represent controller buttons.
- Must visually highlight the current frame being executed by the emulator.
- Must allow the user to toggle (add/remove) button inputs for any given frame by clicking on the corresponding grid cell.
- Must allow saving the modified input sequence back to a `.tas.json` file.

## 🚫 Out of Scope
- Multi-controller (Player 2+) editing support (Phase 2).
- Advanced macro recording or input generation scripting within the UI.
- Audio waveform visualization overlaid on the input timeline.
