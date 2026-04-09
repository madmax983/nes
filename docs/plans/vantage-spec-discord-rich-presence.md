# 🔭 Vantage: Spec for Discord Rich Presence

## 👤 User Story
As a Netplay Player, I want my Discord profile to show which NES game I am currently playing and whether I have an open Netplay room, so that my friends can easily see my status and ask to join my session without me manually messaging them.

## 💼 Business Problem (So What?)
Multiplayer emulation is inherently social, but discovering that a friend is looking for a player is currently an out-of-band, manual process. By integrating with Discord Rich Presence, we turn every active player into a billboard for the emulator. This drives organic growth, lowers the friction for joining netplay sessions, and increases overall engagement.

## 📈 Success Metrics
- **Engagement:** 20% increase in daily active Netplay sessions.
- **Acquisition:** Measure the number of times users click "Ask to Join" on a rich presence profile.
- **Adoption:** 50% of desktop users have Rich Presence enabled.

## 🕵️ Gap Analysis
- **Market View:** Modern standalone emulators (like Dolphin, RetroArch) and standard PC games heavily utilize Discord Rich Presence to show current game, elapsed time, and multiplayer status.
- **Our Gap:** Our emulator has a robust rollback netcode engine but zero social presence. Players must copy-paste room names in DMs.

## ✅ Acceptance Criteria
- The desktop emulator must connect to the local Discord client via IPC on startup (if enabled).
- Must display the currently loaded ROM name in the Discord profile.
- Must display the elapsed time since the ROM was loaded.
- If a Netplay session is active, the presence must show the current Netplay Room Name and player count (e.g., "1/2 Players").
- Users must be able to disable Discord Rich Presence via the `nes.toml` configuration file.
- The emulator must not crash or degrade performance if the Discord client is closed or unavailable.

## 🚫 Out of Scope
- Support for Discord Rich Presence in the Web (Trunk) or TUI adapters.
- Implementing a full Discord "Invite to Game" button that automatically launches the emulator (requires deep OS protocol handlers). Phase 1 is purely informational.
