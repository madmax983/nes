# 🔭 Vantage: Spec for Cross-Platform Cloud Saves

## 👤 User Story
"As a Player who uses multiple devices, I want my save states and SRAM data to automatically sync to the cloud, so that I can seamlessly continue my game from my desktop to my web browser without manually transferring files."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Our emulator runs on Desktop, Web, and TUI. However, player progression is currently siloed to the local machine (or browser's IndexedDB). This forces users to either manually move `.state.json` files around or abandon their progress when switching devices. By implementing Cloud Saves, we unify the ecosystem, increasing user retention and making the web client a viable continuation of the desktop experience rather than an isolated toy.

## 📊 Success Metrics
- **Reliability:** 99.9% of cloud save uploads/downloads complete without data corruption.
- **Speed:** Sync completes in under 2 seconds on a standard broadband connection.
- **Adoption:** 25% of active users link a cloud provider (e.g., Google Drive, Dropbox) or a dedicated relay account within the first month.

## 🕵️ Gap Analysis
- **Market View:** Premium emulators and modern gaming platforms (Steam, RetroArch via external tools) offer seamless cloud saves, setting player expectations for cross-device continuity.
- **Our Gap:** We currently support local savestates (F5/F8) and Web host ROM persistence, but lack any network-based persistence layer for saves/SRAM.

## ✅ Acceptance Criteria
- Must provide an interface in the Desktop and Web clients to authenticate with a cloud storage provider (or our own relay service).
- Must automatically upload SRAM (battery-backed memory) on game exit or when periodically flushed.
- Must automatically check for and download newer cloud saves on game load.
- Must gracefully handle sync conflicts (e.g., prompt the user to choose between Local and Cloud if both were modified offline).

## 🚫 Out of Scope
- Real-time multiplayer synchronization (already handled by Netplay).
- Syncing ROM files themselves (users must provide the ROM locally on each device).
- Syncing RTA speedrun profiles or input macro logs (Phase 2).
