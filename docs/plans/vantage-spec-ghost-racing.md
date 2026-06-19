# 🔭 Vantage: Spec for Asynchronous Ghost Racing

## 👤 User Story
"As a Speedrunner, I want to see a 'ghost' of my personal best or a downloaded TAS run while I play, so that I can visually compare my current pace against the target time without taking my eyes off the game."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our RTA (Real-Time Attack) mode is heavily focused on timing splits and writing performance metrics. However, speedrunners rely heavily on visual cues. They often use external video players to compare runs. By utilizing our existing `nes_core::tas` recorded run-length movies, we can overlay a "Ghost" representation. This increases the sticky value of our RTA mode, keeping competitive players in our ecosystem and transforming the emulator into an indispensable training tool.

## 📊 Success Metrics
- **Performance:** Rendering the ghost overlay adds less than 1ms to the frame rendering time and maintains 60fps.
- **Utility:** A user can load a `.tas.json` file as a ghost and see it replay concurrently with their live gameplay.
- **Adoption:** 30% of RTA mode calibration runs are later used as ghosts for subsequent attempts.

## 🕵️ Gap Analysis
- **Market View:** Other emulators rely on external tools or specialized forks (like practice ROMs) to provide ghost functionalities. Some modern official retro releases (like Nintendo Switch Online) provide ghosts for specific games, but general NES emulators lack native support.
- **Our Gap:** We already generate and replay TAS artifacts deterministically via `nes_core::tas`. We just lack the visual overlay to blend a background emulation instance's output with the live foreground instance.

## ✅ Acceptance Criteria
- Must allow a user to select a `*.tas.json` run file to act as the "Ghost".
- Must render the ghost run concurrently, overlaying its video output with a configurable opacity/transparency over the live game.
- Must synchronize the start of the ghost run with the start of the live run (e.g., waiting for the first user input).
- Must pause or stop the ghost if the live game is paused or reset.

## 🚫 Out of Scope
- Leaderboard integration or downloading ghosts directly from a server (Phase 2).
- Multiplayer real-time racing (handled by Netplay).
