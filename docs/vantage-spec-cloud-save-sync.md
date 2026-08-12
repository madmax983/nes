# 🔭 Vantage: Spec for Cloud Save Sync

## 👤 User Story
"As a multi-device Gamer, I want my game saves (battery RAM) and savestates to automatically synchronize across the desktop and web versions of the emulator, so that I can start a game on my PC and seamlessly continue it on my phone or laptop."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
While our emulator boasts high accuracy and experimental features like rollback netplay and AI training, user retention across different platforms (Desktop vs Web) is hindered by fragmented save data. Users currently have to manually export and import save files when moving between the desktop app and the web demo. By implementing Cloud Save Sync, we eliminate this friction, transforming the emulator from a disparate set of isolated applications into a unified, ecosystem-driven platform. This directly increases daily active users (DAU) on the web platform by providing a frictionless continuation of desktop play sessions.

## 📊 Success Metrics
- **Performance:** Sync operations occur in the background without causing frame drops or audio stuttering during gameplay.
- **Utility:** A user can save on Desktop and load that exact state on the Web client within 10 seconds of opening the browser.
- **Adoption:** 20% of active users link an account/storage provider to enable sync within the first month.

## 🕵️ Gap Analysis
- **Market View:** Leading emulators offer cloud sync, and commercial platforms make cross-device cloud saves a flagship, expected feature.
- **Our Gap:** Save data (Battery RAM) and manual savestates are currently written exclusively to the local filesystem (Desktop) or local web storage (Web). There is no mechanism to bridge these environments.

## ✅ Acceptance Criteria
- Must provide an authentication UI or OAuth flow to link the emulator to a cloud storage provider.
- Must automatically upload Battery RAM and manual savestates when created or updated.
- Must automatically download and restore the latest cloud saves upon launching a ROM, if the cloud version is newer than the local version.
- Must handle conflicts gracefully (e.g., prompting the user if both local and cloud saves have diverged).
- Must be available on both the Desktop (`nes-desktop`) and Web (`nes-web`) clients.

## 🚫 Out of Scope
- Syncing large AI training artifacts, TAS run records, or ROM files themselves.
- Real-time multiplayer synchronization (already handled by rollback netplay).
- Phase 1: Self-hosted sync server.
