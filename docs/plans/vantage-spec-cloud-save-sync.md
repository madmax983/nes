# 🔭 Vantage: Spec for Cloud Save Sync

## 👤 User Story
"As a Multi-Device Player, I want my game saves and savestates to automatically sync to the cloud, so that I can seamlessly continue my progress on my desktop after playing on the web."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, our emulator isolates game saves (battery-backed RAM) and savestates to the local environment (e.g., IndexedDB on the web, local files on desktop). Users who play across multiple platforms (e.g., trying a ROM on their phone's browser, then moving to a desktop for a longer session) experience friction because their progress is trapped. By providing seamless cloud save sync, we unify the user experience across all our runtime adapters (`nes-web`, `nes-desktop`, `nes-tui`), increasing user retention and encouraging longer, multi-session engagement within our ecosystem.

## 📊 Success Metrics
- **Performance:** Syncing operations happen asynchronously and do not stall the core emulation loop or introduce frame drops.
- **Utility:** A user can create a savestate on `nes-web` and load it on `nes-desktop` within 5 seconds of the initial save.
- **Adoption:** 20% of users who play on more than one platform enable cloud save sync.

## 🕵️ Gap Analysis
- **Market View:** Modern commercial emulators and retro gaming platforms (like RetroArch via cloud storage, or Nintendo Switch Online) offer automatic cloud backups and seamless cross-device syncing.
- **Our Gap:** We have functional save systems in both desktop and web adapters, but they are strictly localized. We lack a unified remote storage backend and the synchronization logic to detect, upload, and merge remote saves with local state.

## ✅ Acceptance Criteria
- Must provide an opt-in configuration option in `nes.toml` (desktop) and UI (web) to authenticate and enable cloud sync.
- Must automatically upload battery-backed SRAM (game saves) when the game saves to it (or upon closing the session).
- Must automatically sync manual savestates (F5/F8 equivalents) across authorized devices.
- Must gracefully handle offline scenarios by queueing local saves and resolving conflicts using a "latest timestamp wins" strategy when reconnected.
- Must ensure save files are cross-compatible between `nes-desktop`, `nes-web`, and `nes-tui`.

## 🚫 Out of Scope
- Syncing input mapping profiles or emulator settings (saves/savestates only for Phase 1).
- Real-time multiplayer synchronization (already handled by `nes-netplay`).
- Version control or deep history of saves (only keeping the latest sync state).
