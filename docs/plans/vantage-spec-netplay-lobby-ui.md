# 🔭 Vantage: Spec for Netplay Lobby UI

## 👤 User Story
As a Multiplayer Gamer, I want an interactive Netplay Lobby overlay, so that I can easily discover, host, and join online game sessions without having to copy-paste room codes or use command-line arguments.

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Currently, Netplay sessions require players to manually share IP addresses, relay ports, and room names via external communication tools (like Discord) and pass them as command-line arguments (`--netplay-relay <host> --netplay-room <name>`). This creates massive friction, preventing spontaneous multiplayer sessions and alienating users who are not comfortable with CLIs. By providing a built-in lobby UI to list active public rooms or host private ones via an easy interface, we democratize access to rollback netplay, driving concurrent user engagement and making our emulator a viable social platform.

## 📈 Success Metrics
- **Adoption:** 50% of Netplay sessions are initiated via the Lobby UI rather than CLI arguments.
- **Usability:** A user can successfully host a public room and a second user can join it within 15 seconds entirely from the GUI.
- **Reliability:** The lobby list accurately reflects the active rooms on the configured relay server with < 5 seconds latency.

## 🕵️ Gap Analysis
- **Market View:** Top-tier multiplayer emulators (like Fightcade or RetroArch's netplay lobby) provide built-in server browsers, allowing players to instantly jump into games with others.
- **Our Gap:** We have a robust, determinism-tested rollback engine (`nes-netplay`) and a room relay server (`nes-relay`), but absolutely no GUI to bridge players to these servers. It is entirely CLI-driven.

## ✅ Acceptance Criteria
- Must add a new "Netplay Lobby" overlay accessible from the main emulator menu.
- Must display a list of active, public rooms retrieved from the configured relay server, showing the game name (ROM hash), host latency (ping), and player count.
- Must provide a "Host Room" button that prompts for an optional password (for private rooms) and automatically registers the session on the relay.
- Must allow joining a listed public room with a single click/button press, automatically configuring the local rollback engine and downloading/verifying the matching ROM if necessary.
- Must include a manual "Join via Code" option for private, unlisted rooms.

## 🚫 Out of Scope
- In-game text chat or voice chat (Phase 2).
- Matchmaking queues or ranked leaderboards.
- Cross-game netplay (both players must have the same ROM hash, as handled by the current engine).
