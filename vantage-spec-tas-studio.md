# 🔭 Vantage: Spec for Interactive TAS Studio

## 👤 User Story
"As a Tool-Assisted Speedrunner, I want a visual piano-roll editor and timeline scrubber, so that I can graphically edit inputs frame-by-frame, test segment outcomes instantly, and stitch together optimal runs without relying on raw text scripts."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Our current TAS movie/recorder primitives record runs deterministically, but modifying runs requires editing the raw automation scripts or formats directly. This creates high friction and locks out visual learners and casual routing experiments. By introducing an interactive TAS Studio (a visual timeline editing suite), we transform the emulator from a mere recording device into a full production suite, capturing the rapidly growing speedrunning and routing community directly in our ecosystem instead of relying on external tooling.

## 📊 Success Metrics
- **Utility:** A user can insert a single frame of right-dpad input into frame 1500 of an existing movie file and replay the result visually within 3 seconds.
- **Performance:** Editing a 2-hour movie file (hundreds of thousands of frames) incurs no UI lag during scrubbing or zooming.
- **Adoption:** 20% of users who load standard TAS ROM suites utilize the TAS Studio feature within a week of release.

## 🕵️ Gap Analysis
- **Market View:** Specialized TAS emulators (like FCEUX or BizHawk) have established "TAS Studio" interfaces featuring piano rolls, branch management, and RAM watching.
- **Our Gap:** We possess a deeply deterministic core and stable movie primitives, which is half the battle. However, we have zero UI to visualize, slice, or edit this data interactively, leaving users to write blind automation scripts.

## ✅ Acceptance Criteria
- Must provide a dedicated timeline UI (the "Piano Roll") displaying frames on the Y-axis and controller inputs on the X-axis.
- Must support zooming and scrolling through the timeline.
- Must allow toggling individual inputs on/off at specific frames with a mouse click.
- Must support "scrubbing" the emulator state backwards and forwards visually based on timeline position, integrating with our existing rewind/save-state capabilities.
- Must allow saving the modified timeline back to standard TAS format.

## 🚫 Out of Scope
- Multi-track branching/merging ("TAS branches" or "Movie states") within the UI for Phase 1.
- Automated brute-forcing or bot integration directly from the UI.
