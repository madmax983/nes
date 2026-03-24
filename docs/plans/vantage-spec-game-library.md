# 🔭 Vantage: Spec for In-Game Game Library / ROM Browser

## 👤 User Story
"As a Player, I want an integrated Game Library to browse and select my ROMs from within the emulator, so that I don't have to navigate through my OS file system every time I want to switch games."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Currently, users are forced to rely on the native OS file picker to load ROMs. This pulls the user out of the application experience, especially when playing full-screen or with a gamepad. An in-game library keeps the user engaged within the emulator's ecosystem, standardizes the UI across desktop platforms, and reduces the friction of discovering and loading different games, leading to longer play sessions.

## 📈 Success Metrics
- **Adoption:** 80% of users launch games from the in-game Game Library instead of the native OS file picker after their first session.
- **Usability:** Players can navigate the library and launch a game entirely using a gamepad without touching the keyboard or mouse.
- **Performance:** Scanning a directory of 1,000 ROMs populates the UI list in under 50ms without causing visual stutter in the user interface.

## 🕵️ Gap Analysis
- **Market View:** Nearly all modern retro gaming platforms (e.g., RetroArch, EmulationStation, DuckStation) feature built-in game browsers with gamepad support, establishing this as a baseline user expectation.
- **Our Gap:** Our emulator forces the user to interact with standard OS dialog boxes. We lack an immersive, controller-friendly method for users to select and load their games.

## ✅ Acceptance Criteria
- Must provide a "Game Library" view accessible from the main overlay menu.
- Must pause the current emulation session when the library view is active.
- Must allow the user to configure a "ROM Directory" path in the configuration file that the library scans on startup.
- Must display a vertically scrollable list of all `.nes` files found in the configured directory, showing their filenames (without the extension).
- Must allow navigating the list using D-pad/Arrow keys and selecting a game using the primary action button.
- Must immediately load and launch the selected game upon confirmation, replacing the currently running ROM.
- Must handle missing files or unreadable directories gracefully with a clear in-game error message.

## 🚫 Out of Scope
- Fetching and displaying box art, screenshots, or metadata from external scraping APIs (e.g., TheGamesDB).
- Support for nested subdirectories (flat folder scanning only for Phase 1).
- Zipped ROM support (`.zip` extraction).