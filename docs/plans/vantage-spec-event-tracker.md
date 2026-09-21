# 🔭 Vantage: Spec for Game Event Tracker

## 👤 User Story
"As a TASer, Speedrunner, or AI Trainer, I want to define memory conditions (like health dropping, score increasing, or level changing) and have the emulator automatically log when those events occur, so that I can easily analyze runs or train agents without manually scrubbing through footage."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Analyzing NES gameplay typically requires either manual visual inspection or constantly polling memory every frame, which is computationally expensive and difficult to scale. By exposing our `EventTracker` to define triggers (e.g., Address $075A changes value), we create a powerful foundation for speedrun verification, automated highlight generation, and AI reinforcement learning rewards. This makes our emulator the preferred backend for any automated analysis of NES games.

## 📊 Success Metrics
- **Performance:** Tracking 10 active triggers adds negligible overhead to the frame time.
- **Utility:** A user can easily configure a trigger for "Score Increased" and receive an event log populated with exact frames/values.
- **Adoption:** Used as the primary mechanism for defining reward functions in the `nes-ai` training pipeline.

## 🕵️ Gap Analysis
- **Market View:** Some emulators support Lua scripts where users can write their own tracking logic, but few have a built-in, performant, declarative event system that can easily output to structured logs (JSON/CSV) or be queried by an external API.
- **Our Gap:** We have the `EventTracker` and `Trigger` structures in `nes-core`, but they are not exposed via any configuration file, API endpoint, or UI in the desktop client to allow users to actually use them.

## ✅ Acceptance Criteria
- Must allow users to define `Triggers` (Address and target Value/Change) via a configuration file (e.g., `events.toml`) or a UI menu.
- Must evaluate these triggers continuously during emulation (e.g., via a post-frame hook).
- Must record generated `Events` to an in-memory log that can be exported (e.g., to a JSON file or terminal output).
- Must support triggers based on "Value Changed", "Value Equaled", and "Value Increased/Decreased".
- Must integrate cleanly with the existing TAS recording system, optionally stamping events onto specific frames in the run.

## 🚫 Out of Scope
- Complex logic expressions (e.g., "Trigger if Address A == X AND Address B == Y") for Phase 1.
- Automatically finding memory addresses (this relies on the user or the Cheat Finder to know the address beforehand).
