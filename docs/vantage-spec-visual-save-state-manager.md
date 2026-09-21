# 🔭 Vantage: Spec for Visual Save State Manager

## 👤 User Story
"As a Player, I want a visual save state manager with multiple slots and screenshot previews, so that I can easily save, identify, and load different points in the game without accidentally overwriting my single quicksave slot."

## ❓ So What? (Business Problem)
Currently, players only have a single "quicksave" slot (F5) which is easily overwritten by accident, leading to lost progress and frustration. Providing a visual manager with multiple slots and screenshots increases player confidence, reduces support complaints about lost saves, and modernizes the emulator experience to match user expectations from other software.

## 📊 Success Metrics
- **Adoption:** 60% of users who use save states utilize the visual manager instead of only relying on the F5/F8 quick keys.
- **Reduction in Data Loss:** Zero reports of users accidentally overwriting their primary save state slot.

## 🕵️ Gap Analysis
- **Market View:** Other modern emulators provide a robust, multi-slot save state manager with visual thumbnails and timestamps.
- **Our Gap:** We only provide a single quicksave slot bound to a hotkey, with no visual feedback on what point in the game was saved.

## ✅ Acceptance Criteria
- Must provide an in-game UI overlay to manage save states.
- Must support at least 10 independent save state slots per ROM.
- Must capture and display a thumbnail screenshot of the emulator output when a save state is created.
- Must display the date and time the save state was created.
- Must allow users to load, overwrite, or delete a specific save state slot.

## 🚫 Out of Scope
- Cloud syncing of save states across devices.
- Auto-saving at regular intervals.
