# 🔭 Vantage: Spec for Bundled ROM Fallback

## 👤 User Story
"As a new User, I want the emulator to automatically load a bundled demonstration ROM when I launch it without any arguments, so that I can immediately see the emulator working without having to source my own games first."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
Currently, users who run the emulator without a valid ROM path (e.g., running the bare executable) are met with an OS error (`No such file or directory (os error 2)`). This poor first-time user experience causes immediate abandonment ("If I can't copy-paste and run it, I'm out!" as noted in ECHO_REPORT.md). By defaulting to our bundled homebrew ROM, we guarantee a successful zero-configuration launch, demonstrating value and keeping the user engaged.

## 📈 Success Metrics
- **FTUE Success:** 100% of zero-argument launches result in a playable game screen instead of a terminal crash.
- **Retention:** Decrease the drop-off rate of users who quit within the first 10 seconds of running the desktop binary by 50%.

## ✅ Acceptance Criteria
- If `nes-desktop` is launched without a ROM path argument, it must automatically load `roms/homebrew/homebrew.nes`.
- It must gracefully log an informative message indicating that it is falling back to the bundled ROM.
- If the user explicitly provides a ROM path that does not exist, it should still fail but with a user-friendly error message, not a raw `os error 2`.

## 🚫 Out of Scope
- Downloading ROMs from the internet if none are found locally.
- A complex GUI file picker on empty launch.
