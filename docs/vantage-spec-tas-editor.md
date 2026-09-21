# 🔭 Vantage: Spec for TAS Editor

## 👤 User Story
"As a Tool-Assisted Speedrunner (TASer), I want a visual piano roll editor for controller inputs, so that I can precisely modify, insert, and delete individual frame inputs without manually editing `*.tas.json` files."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
We currently have a stable TAS recording and playback foundation in `crates/nes-core` that generates artifacts for testing and AI evaluation. However, modifying these runs requires cumbersome manual text editing. By providing a visual editor, we empower human creators to iteratively craft and optimize tool-assisted speedruns directly within our environment, increasing the utility of our TAS engine beyond just AI evaluation.

## 📊 Success Metrics
- **Utility:** A user can insert a single-frame controller input into an existing run and resume playback from that point.
- **Adoption:** 25% of users who record a TAS movie utilize the editor to modify it.

## 🕵️ Gap Analysis
- **Market View:** Other platforms provide comprehensive piano roll editors and re-recording capabilities.
- **Our Gap:** We have the backend primitive, but zero UI for human editing. It is strictly an automation/AI tool right now.

## ✅ Acceptance Criteria
- Must provide a UI window displaying a scrolling list of frames and controller inputs.
- Must allow clicking to toggle input states for specific controller buttons on specific frames.
- Must support inserting and deleting frames, dynamically adjusting the movie length.
- Must allow seeking the emulator state to a specific frame clicked in the editor.
- Must save modifications back to the `*.tas.json` format.

## 🚫 Out of Scope
- Multi-track branching/version control (Phase 2).
- Support for 4-player multiplayer TAS editing (Phase 2).
