# 🔭 Vantage: Spec for Cloud Save Sync

## 👤 User Story
As a multi-device player, I want my save data and RTA profiles to automatically sync between the desktop emulator and the web emulator, so that I can seamlessly continue my game regardless of the device I am using.

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Our emulator is currently fragmented across multiple platforms (Desktop, Web, TUI). A user playing on their desktop cannot easily resume their progress on the web version without manually transferring their save files. By implementing a unified Cloud Save Sync feature (e.g., via our existing relay infrastructure or a lightweight backend), we increase user retention across platforms and provide a modern, seamless experience that sets us apart from strictly offline emulators.

## 📈 Success Metrics
- **Reliability:** 99.9% of saves successfully synchronize without corruption.
- **Latency:** Cross-platform sync completes within 2 seconds of a state save.
- **Adoption:** 30% of users who play on both web and desktop link their accounts/devices for syncing.

## 🕵️ The Reality:
- **Market View:** Modern gaming platforms (Steam, RetroArch via cloud plugins, Nintendo Switch Online) all provide seamless save data roaming. Players expect progress to travel with them.
- **Our Gap:** We have robust save serialization and Web storage, but zero connective tissue. Files remain trapped on the local filesystem or browser storage.

## ✅ Acceptance Criteria
- Must provide a mechanism (e.g., a simple pairing code or OAuth) to link a Web session with a Desktop profile.
- Must automatically upload manual saves and RTA profiles to the sync service.
- Must poll or receive push notifications for updated saves upon launch or via a manual "Sync" button.
- Must resolve conflicts gracefully (e.g., prompt the user if local and remote saves diverge).
- Must handle offline scenarios without blocking emulation or panicking.

## 🚫 Out of Scope
- Real-time multiplayer synchronization (already handled by netplay).
- Syncing large ROM files themselves (users must provide the ROM locally on both ends).
