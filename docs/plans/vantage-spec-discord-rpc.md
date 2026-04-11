# 🔭 Vantage: Spec for Discord Rich Presence

## 👤 User Story
"As a Social Gamer, I want my Discord status to show what NES game I am playing, so that my friends can see my activity and ask to join me via Netplay."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Emulator growth relies heavily on word-of-mouth. Currently, gameplay happens in isolation. By automatically broadcasting gameplay to Discord, we turn every active user into a free billboard for the emulator, driving organic discovery and increasing awareness of our Netplay capabilities.

## 📈 Success Metrics
- **Adoption:** 10% of active desktop users have Discord Rich Presence enabled.
- **Engagement:** 5% increase in Netplay sessions initiated by users discovering the emulator through Discord statuses.

## 🕵️ Gap Analysis
- **Market View:** Standard feature in top-tier emulators (e.g., RetroArch, Dolphin).
- **Our Gap:** We currently have zero social presence or external visibility when a user is running the emulator.

## ✅ Acceptance Criteria
- Must detect if the Discord desktop client is running.
- Must display the cleaned title of the currently loaded ROM (e.g., "Super Mario Bros.") in the Discord activity status.
- Must display the elapsed playtime for the current session.
- Must display "In Menus" or "Idle" when no game is loaded.
- Must provide a configuration toggle to allow users to opt-out of sharing their activity.
- Must gracefully handle Discord disconnecting or not being present without crashing the emulator.

## 🚫 Out of Scope
- Rich Presence "Ask to Join" or "Spectate" buttons for direct Netplay launching (Phase 2).
- Uploading custom game box art assets to Discord (will use a default emulator icon for now).
