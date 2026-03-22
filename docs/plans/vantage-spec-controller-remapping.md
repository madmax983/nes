# 🔭 Vantage: Spec for In-Game Controller Remapping UI

## 👤 User Story
"As a Player, I want to remap keyboard and gamepad inputs using an in-game menu, so that I can customize my controls without having to manually edit the `nes.toml` configuration file."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Currently, users are forced to edit the `nes.toml` file to change their control scheme. This creates a significant barrier to entry for casual users who are accustomed to modern, accessible UI menus. By allowing in-game remapping, we reduce user friction, decrease the abandonment rate during initial setup, and provide an expected standard feature that elevates the perceived quality and polish of the emulator.

## 📈 Success Metrics
- **Usability:** 90% of users can successfully remap at least one button within 15 seconds of opening the menu.
- **Engagement:** Decrease the number of issues/questions related to "how do I change controls" to near zero.
- **Robustness:** Zero crashes or conflicting mappings (e.g., mapping two different actions to the same button) allowed by the UI without a clear warning.

## ✅ Acceptance Criteria
- Must provide a "Controls" section within the existing in-game overlay menu (accessed via `Escape`).
- Must pause the emulator core while the remapping menu is open.
- Must display the current mappings for both Player 1 and Player 2 (Keyboard and Gamepad).
- Must allow the user to select an action (e.g., "A Button", "D-Pad Up") and prompt them to "Press a key/button".
- Must listen for the next keyboard or gamepad input and assign it to the selected action.
- Must immediately persist the new mappings to `nes.toml` so they survive restarts.
- Must provide a "Reset to Defaults" button to restore the original control scheme.
- Must prevent or warn the user if they attempt to map a key/button that is already assigned to a critical system function (like `Escape` for the menu, or `F5` for quicksave).

## 🚫 Out of Scope
- Support for complex macros (mapping one button to multiple actions, e.g., "A+B").
- Turbo/Autofire configurations in this phase.
- Support for more than 2 players (e.g., Multitap adapters).
