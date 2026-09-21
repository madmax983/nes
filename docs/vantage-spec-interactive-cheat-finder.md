# 🔭 Vantage: Spec for Interactive Cheat Finder UI

## 👤 User Story
"As a Player, I want an interactive interface to search for and filter memory values while playing, so that I can create my own cheat codes (like infinite lives or health) for games."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, we have experimental underlying core support for searching and filtering memory values to find specific game states. However, this engine is entirely headless. Players must use pre-existing Game Genie codes (via `SessionCheats`) and cannot easily discover new ones for unmapped games or homebrew. By building an interactive UI for the cheat finder module, we empower players to customize their own experience and bypass arbitrary difficulty walls, directly competing with features found in established emulators and dedicated cheat software.

## 📊 Success Metrics
- **Performance:** Activating the cheat finder UI adds no perceptible input lag during normal gameplay.
- **Utility:** A user can reliably isolate a specific memory address (e.g., lives count) within 5 filter iterations.
- **Adoption:** 20% of users who load a ROM engage with the Cheat Finder UI to search for values.

## 🕵️ Gap Analysis
- **Market View:** Classic emulators and memory editors provide robust, interactive ways to search for unknown memory values by performing successive "Changed/Unchanged" or "Exact Value" scans.
- **Our Gap:** We implemented the core logic (`crates/nes-core/src/experimental/cheat_finder.rs`) but left it as an API without any user-facing integration in `nes-desktop`. The feature is essentially dead weight until exposed.

## ✅ Acceptance Criteria
- Must provide a UI window or overlay to start a new memory search.
- Must allow the user to filter the current candidates by: Exact Value, Not Equal to Value, Changed, and Unchanged.
- Must display the number of remaining memory candidates after each filter pass.
- Must allow the user to easily generate a cheat code or lock the value of an identified memory address directly from the UI.

## 🚫 Out of Scope
- Support for searching external cartridge RAM or save data for Phase 1.
- Advanced multi-byte (16-bit/32-bit) value searching.
