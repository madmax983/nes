# 🔭 Vantage: Spec for Multi-Track TAS Merging

## 👤 User Story
"As a Tool-Assisted Speedrunner, I want to merge multiple TAS input timelines (e.g., separating movement and action tracks, or splicing segments from different runs), so that I can construct a flawless macro sequence without destructively overwriting my entire run."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, creating a TAS relies on single, linear recording sessions. If a user wants to fix a single frame or combine the perfect platforming of Run A with the perfect boss fight of Run B, they must manually edit JSON artifacts or start over. This makes complex run creation tedious. By providing non-destructive timeline merging, we elevate the emulator to a professional TAS workstation, drastically reducing the time-to-production for high-tier speedruns and increasing engagement with our `nes-ai` / TAS tooling.

## 📊 Success Metrics
- **Performance:** Timeline merges process in under 500ms for a 1-hour run.
- **Utility:** Users can successfully combine two distinct runs without losing synchronization or inputs.
- **Adoption:** 20% of TAS creators utilize the merge functionality in their workflow.

## 🕵️ Gap Analysis
- **Market View:** Existing TAS tools rely heavily on external text editors or clunky third-party GUIs for splicing inputs.
- **Our Gap:** We have the primitive `nes_core::tas` foundations, but we strictly record linear tracks and lack internal tooling to splice, branch, or merge input streams programmatically.

## ✅ Acceptance Criteria
- Must support combining inputs from two different timeline files (`*.tas.json`).
- Must allow defining a specific "splice frame" where Timeline A ends and Timeline B begins.
- Must support logical OR combinations of inputs on the same frame (e.g., merging a track with only D-Pad inputs and a track with only A/B inputs).
- Must generate a valid, runnable, composite TAS artifact.
- Must expose this capability via a CLI subcommand in `nes-mcp` or `nes-ai`.

## 🚫 Out of Scope
- A full graphical visual timeline editor (Phase 2).
- Real-time, mid-emulation splicing (this is an offline artifact processing task for Phase 1).
