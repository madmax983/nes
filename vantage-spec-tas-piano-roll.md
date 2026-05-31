# 🔭 Vantage: Spec for TAS Piano Roll Editor

## 👤 User Story
"As a TAS (Tool-Assisted Speedrun) Creator, I want an interactive piano roll editor for controller inputs, so that I can visualize, scrub, and surgically modify my input sequence frame-by-frame without manually editing JSON files."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our `nes-core` supports deterministic TAS replay and `nes-ai` generates TAS artifacts (`*.tas.json`). However, there is no integrated tool to visualize or edit these input sequences within the emulator. Creators must either rely on the legacy macro script format or manually edit raw JSON, which is error-prone and tedious. By providing a native piano roll editor, we elevate our emulator from a simple replay tool to a full-fledged speedrunning workstation. This bridges the gap between AI-generated runs and human optimization, making the toolchain sticky and indispensable for the speedrunning community.

## 📊 Success Metrics
- **Performance:** Editing a 1-hour input sequence (216,000 frames) causes zero noticeable UI lag.
- **Utility:** A user can insert a single A-button press at frame 1500 and immediately resume replay from that frame using a transparent savestate rewind/fast-forward.
- **Adoption:** 80% of users who load a `*.tas.json` artifact interact with the piano roll editor.

## 🕵️ Gap Analysis
- **Market View:** Industry-standard TAS emulators (like FCEUX or BizHawk) feature mature piano roll editors that allow inserting, deleting, and modifying inputs on a per-frame basis.
- **Our Gap:** We have the underlying deterministic core (`nes_core::tas`) and input recording/playback logic, but lack the frontend UI to visualize or mutate the tape.

## ✅ Acceptance Criteria
- Must display a scrolling vertical or horizontal timeline representing frames.
- Must visualize button states (A, B, Select, Start, Up, Down, Left, Right) for each frame.
- Must allow users to click/drag to toggle button states on specific frames.
- Must allow inserting or deleting empty frames to shift the timeline.
- Must sync the emulator's current execution frame with the piano roll's playhead.

## 🚫 Out of Scope
- Multi-controller (Player 2/3/4) support in the initial UI (Phase 2).
- Automatic macro/script generation from the UI (Phase 2).
- Real-time collaborative editing (Netplay for TAS).
