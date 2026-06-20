# 🔭 Vantage: Spec for Input Remapping UI

## 👤 User Story
"As a Player, I want to be able to remap the keyboard and gamepad inputs to different buttons within the emulator's UI, so that I can play comfortably with my preferred control layout without modifying configuration files directly."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, the input mapping (e.g., Z for A, X for B, Arrow keys for D-Pad) is hardcoded in `map_virtual_keycode` (in `input.rs`). While we have a robust emulator, players with different keyboard layouts (e.g., AZERTY, Dvorak) or accessibility needs are forced to use our arbitrary defaults or modify the source code and recompile. A customizable input remapping UI significantly improves accessibility, player retention, and standardizes our application alongside modern emulator expectations. If a player can't play comfortably, they won't play at all.

## 📊 Success Metrics
- **Utility:** 100% of the standard NES controller inputs (A, B, Select, Start, D-Pad) can be reassigned to any valid keyboard key via the UI.
- **Persistence:** Custom mappings are saved to the configuration file (`nes.toml`) and persist across application restarts.
- **Accessibility:** Players using non-QWERTY layouts can successfully configure the emulator to use their home row keys.

## 🕵️ Gap Analysis
- **Market View:** All mainstream emulators (FCEUX, Mesen, RetroArch) provide a dedicated "Input Configuration" settings menu allowing custom keybinds.
- **Our Gap:** We currently hardcode keyboard mappings in `map_virtual_keycode`. We also lack a UI overlay panel dedicated to settings or configuration changes.

## ✅ Acceptance Criteria
- Must provide a new "Settings" or "Input" tab within the existing emulator overlay menu.
- Must display the current keyboard mappings for all NES inputs (A, B, Select, Start, Up, Down, Left, Right).
- Must allow the user to select an input and press a new key to assign it.
- Must handle duplicate assignments gracefully (e.g., clearing the previous mapping or warning the user).
- Must save the updated mappings to the `[input]` section of `nes.toml` immediately upon change or when closing the menu.
- The `input.rs` translation layer must dynamically read from the active configuration instead of using a hardcoded `match` statement.

## 🚫 Out of Scope
- Gamepad/Controller remapping UI (Phase 1 will focus solely on Keyboard remapping; gamepad will use default standard mapping).
- Support for mapping multiple keys to the same single NES button (e.g., allowing both 'Z' and 'Space' to press 'A' simultaneously).
- Turbo/Autofire configuration in the remapping UI.
