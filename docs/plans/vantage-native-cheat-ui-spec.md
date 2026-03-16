# 🔭 Vantage: Spec for Native In-Engine Cheat UI

## 👤 User Story
As a Desktop Player, I want an in-game UI to toggle Game Genie cheat codes, so that I don't have to close the emulator and manually edit `nes.toml` to try different cheats.

## 💼 Business Problem (So What?)
Manually editing configuration files is a huge friction point for players who just want to experiment with different cheat codes. A built-in UI reduces context switching and keeps players engaged with the emulator, directly increasing session length and overall satisfaction.

## 📈 Success Metrics
- 80% reduction in time taken to apply a cheat code compared to manual configuration edits.
- The UI should not introduce any input lag or performance overhead when rendering over the game.

## ✅ Acceptance Criteria
- Must provide an in-game overlay menu to input and toggle Game Genie codes.
- Must parse standard 6-character and 8-character Game Genie codes.
- Must pause the emulator core while the menu is open.
- Must allow the user to enable or disable individual cheat codes on the fly.
- Must persist entered cheat codes for the loaded ROM to avoid re-entry.
- **Critical Caveat:** Cheats **must be forcibly disabled** during active Netplay sessions or strict RTA mode to prevent unfair advantages.

## 🚫 Out of Scope
- Integration with external cheat databases (e.g., automatically downloading cheat codes from the web).
- Support for complex RAM patch formats other than Game Genie codes.
- Support for Web/Trunk host in this iteration.