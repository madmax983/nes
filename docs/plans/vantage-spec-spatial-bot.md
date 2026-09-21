# 🔭 Vantage: Spec for Spatial Automaton (Bot)

## 👤 User Story
"As a ROM Hacker or AI Researcher, I want to define simple spatial rules (e.g., 'If an object enters this zone, press A for 10 frames'), so that I can create rudimentary gameplay automatons or automated testing scripts without writing complex AI models."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
While our `nes-ai` stack is powerful, training a neural network for simple tasks (like repeatedly jumping over a specific obstacle) is overkill and time-consuming. By exposing the `SpatialBot` and `ZoneTracker`, we allow users to create reactive, rule-based agents easily. This is incredibly valuable for automated QA testing of ROM hacks (e.g., ensuring a jump is possible) or for creating simple "assist" modes for players. It bridges the gap between manual TAS scripting and full AI training.

## 📊 Success Metrics
- **Utility:** A user can successfully configure a bot rule to "Jump when sprite enters X:100-120" and the bot executes the action reliably.
- **Performance:** Evaluating bot rules against zone events adds negligible overhead to frame execution.
- **Adoption:** Used by at least 2 internal test scripts for automated ROM verification.

## 🕵️ Gap Analysis
- **Market View:** Most emulators require external Lua scripting to achieve this level of reactivity. Built-in automation is rare.
- **Our Gap:** The `SpatialBot` exists in `nes-core` but is completely hidden. There is no way for a user to define `BotRule`s or `Zone`s in the desktop client or via the CLI.

## ✅ Acceptance Criteria
- Must allow defining spatial rules (Zone ID, Button, Duration) via configuration (e.g., `bot.toml`).
- Must integrate with the `ZoneTracker` to receive spatial events.
- Must queue and execute the corresponding controller `Command`s (press and release) accurately for the specified duration.
- Must provide a toggle in the UI or CLI to enable/disable the Spatial Bot.
- Must execute deterministically (e.g., if the same seed and bot configuration are used, the output is identical).

## 🚫 Out of Scope
- Complex logic (e.g., AND/OR conditions between multiple zones) for Phase 1.
- Training the bot via reinforcement learning (that is the domain of `nes-ai`).
- Visualizing the active zones on the screen (this could be a Phase 2 addition to the Hitbox Visualizer).
