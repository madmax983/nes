# 🔭 Vantage: Spec for Bundled ROM Fallback

## 👤 User Story
As a new user evaluating the emulator, I want the quickstart command to automatically fall back to the bundled homebrew ROM if my specified ROM path is not found, so that I can immediately see the emulator working without configuring paths.

## 💼 Business Problem (So What?)
First impressions matter. A "File Not Found" error on step 1 of the README causes immediate drop-off. A seamless fallback ensures every user successfully boots the emulator on their first try, building trust and engagement.

## 📈 Success Metrics
- Zero "File Not Found" errors when users copy-paste the `README.md` quickstart commands on a fresh clone.

## 🕵️ Gap Analysis
- Market View: Many emulators boot into a default splash screen, UI, or bundled ROM when no explicit file is given.
- Our Gap: The README examples result in immediate crashes due to hardcoded ROM paths that do not exist on the user's machine, violating out-of-the-box functionality.

## ✅ Acceptance Criteria
- If the user-provided ROM path does not exist, the desktop and netplay applications must automatically attempt to load `./roms/homebrew/homebrew.nes`.
- If the fallback is used, a clear CLI warning/message must be printed indicating that the requested file was not found and the fallback ROM is being used.
- If the fallback ROM *also* does not exist, the original "File Not Found" error for the user's requested path should be preserved and displayed.

## 🚫 Out of Scope
- Automatic downloading of ROMs from the internet.
