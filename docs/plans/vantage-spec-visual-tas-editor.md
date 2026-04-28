# 🔭 Vantage: Spec for Visual TAS Editor

## 👤 User Story
"As a TAS Creator, I want a visual piano-roll interface for editing frame-by-frame inputs, so that I can easily manipulate controller actions, visually align them with game events, and splice runs together without manually editing JSON files."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our `nes_core::tas` module provides a stable foundation for recording and replaying deterministic inputs. However, the creation and manipulation of these TAS files are highly inaccessible, requiring users to either write string-based macros or manually edit complex JSON arrays. By building a Visual TAS Editor directly into the desktop/TUI environment, we drastically lower the barrier to entry for TAS creation. This shifts our product from being merely an execution engine to a full-fledged creative suite, attracting speedrunners and routing enthusiasts who currently rely on older, fragmented tools.

## 📊 Success Metrics
- **Performance:** Editing a 10,000-frame TAS movie introduces no UI lag or stuttering during scrolling or playback.
- **Utility:** A user can successfully insert a 5-frame wait state in the middle of a run without breaking subsequent desync-sensitive actions.
- **Adoption:** 80% of TAS movies produced using our emulator are modified via the visual editor rather than raw JSON edits.

## 🕵️ Gap Analysis
- **Market View:** Legacy TAS tools (like FCEUX or BizHawk) feature robust "Piano Roll" editors, allowing intuitive dragging, dropping, and toggling of inputs across a timeline.
- **Our Gap:** We have the deterministic core (`TasMovie`, `TasRecorder`) but expose zero editing UI. The current workflow expects programmatic generation (via `nes-ai`) or tedious manual file modification, entirely alienating human TAS creators.

## ✅ Acceptance Criteria
- Must provide a timeline-based "piano roll" UI overlay, displaying frames on the Y-axis and controller buttons on the X-axis.
- Must allow users to toggle individual button states on specific frames via point-and-click.
- Must support selecting, copying, and pasting blocks of frames.
- Must support inserting or deleting frames, dynamically shifting the remaining inputs in the timeline.
- Must allow seeking the emulator to the currently selected frame in the editor (scrubbing).
- Must seamlessly serialize back to the `TasMovie` JSON format upon saving.

## 🚫 Out of Scope
- Advanced macro generation directly within the UI (Phase 2).
- Branching timelines / multi-track TAS merging (Phase 2).
- Memory-watch integration within the TAS editor window (handled by CPU debugger).
