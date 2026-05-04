# 🔭 Vantage: Spec for Cloud Save Sync

## 👤 User Story
"As a Multi-Device Gamer, I want my save states to automatically sync to the cloud, so that I can seamlessly transition between playing on my desktop and playing in the web browser."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
User retention is currently fragmented across platforms. Desktop users rarely transition to the web version, and vice-versa, because their progress is locked to a single device. By enabling cloud sync, we remove the friction of platform switching, increasing total engagement time and daily active users across all our clients.

## 📈 Success Metrics
- **Adoption:** 15% of active users link an external cloud account (e.g., Google Drive, Dropbox) or a built-in account.
- **Engagement:** 20% increase in cross-platform usage (users who play on both web and desktop within a 7-day period).

## 🕵️ The Reality:
- **Market View:** Modern gaming ecosystems (Steam Cloud, Nintendo Switch Online) set the baseline expectation that progress follows the user.
- **Our Gap:** Save states and SRAM are entirely local to the device running the emulator. Web saves are trapped in IndexedDB, and desktop saves are trapped in the local file system.

## ✅ Acceptance Criteria
- Must provide an authentication flow to link a cloud storage provider or internal account.
- Must automatically upload local save state files and SRAM data upon saving.
- Must detect and download newer save states from the cloud upon emulator startup or game load.
- Must handle conflict resolution gracefully (e.g., asking the user which save to keep if local and cloud differ).
- Must work across both Desktop and Web platforms.

## 🚫 Out of Scope
- Real-time multiplayer synchronization (already handled by Netplay).
- Syncing large uncompressed capture files, TAS videos, or entire ROM libraries.
