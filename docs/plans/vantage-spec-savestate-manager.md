# 🔭 Vantage: Spec for In-Game Savestate Manager UI

## 👤 User Story
"As a Player, I want an in-game UI to manage multiple savestates for a game, so that I can easily create, label, and load specific moments without managing files manually or remembering hotkeys for different slots."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, users can only quicksave/quickload using `F5`/`F8`, which overwrites a single state file based on the ROM hash. Players who want to practice specific sections, track multiple branching paths, or share states with others have to manually dig into the `./savestates/` directory to backup and rename files. This is a poor user experience. An in-game manager increases the utility and accessibility of the emulator, keeping users engaged with the application rather than their file explorer.

## 📊 Success Metrics
- **Adoption:** 30% of players who use quicksaves migrate to using the Savestate Manager within a month.
- **Usability:** Players can create, rename, and load a state entirely via the gamepad or keyboard without leaving the emulator window.
- **Performance:** Opening the manager UI pauses the game instantly and renders without dropping frames.

## 🕵️ Gap Analysis
- **Market View:** Nearly all modern emulators feature multiple savestate slots (usually 0-9) selectable via hotkeys or an overlay menu.
- **Our Gap:** We only support a single, hardcoded quicksave slot per ROM via F5/F8. We lack a UI overlay for file management.

## ✅ Acceptance Criteria
- Must provide an in-game overlay menu accessible via a hotkey or gamepad combo (e.g., `Escape` or `Start+Select`).
- Must pause the emulator core while the menu is open.
- Must display a list of at least 10 state slots (0-9) specific to the currently loaded ROM.
- Must allow the user to Save, Load, or Delete a state in a selected slot.
- Must show a visual preview (thumbnail screenshot) and timestamp for occupied slots.
- Must persist states to disk in a structured format (e.g., `./savestates/<rom-stem>-<hash8>/slot_<N>.state.json`).
- Must smoothly resume emulation when the menu is closed.
- Must clearly indicate an error if loading a corrupted or incompatible state.

## 🚫 Out of Scope
- Cloud syncing of savestates.
- Advanced state editing (e.g., manually changing variables inside the JSON state).
- A full VFS (Virtual File System) for savestates.
- Support for more than 10 slots in Phase 1 (slots 0-9 only).
