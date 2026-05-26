# 🔭 Vantage: Spec for Out of the Box Experience

## 👤 User Story
"As a new user trying the emulator for the first time, I want to launch the desktop app without providing any command-line arguments and immediately see a working game, so that I don't have to hunt for ROM files or read setup documentation just to verify it works."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
Currently, users who run `cargo run -p nes-desktop` without arguments are greeted with an empty window or an error, forcing them to find a ROM and learn the CLI syntax. As noted in `ECHO_REPORT.md`, even our documentation examples assume the user has configured paths correctly. By automatically loading our built-in `homebrew.nes` when no ROM is provided, we eliminate friction during the critical "Time to First Play" (TTFP) window. This improves the onboarding experience, reduces support queries, and immediately demonstrates the emulator's capabilities.

## 📊 Success Metrics
- **Time to First Play (TTFP):** Reduced to < 5 seconds for first-time builders.
- **Conversion:** 90% of new users successfully see gameplay on their first launch without reading the README.

## 🕵️ Gap Analysis
- **Market View:** Polished commercial software and leading emulators (like RetroArch) often provide default "core" content or a highly guided UI on first launch.
- **Our Gap:** We have a great `homebrew.nes` included in the repo (`roms/homebrew/homebrew.nes`), but we require the user to explicitly pass its path. We lack a fallback mechanism in `nes-desktop`'s CLI parser.

## ✅ Acceptance Criteria
- Must detect when `nes-desktop` is launched without a positional ROM path argument.
- Must automatically resolve and load the bundled `roms/homebrew/homebrew.nes` relative to the workspace root in this scenario.
- Must display a brief, non-intrusive notification or log message indicating that the default homebrew ROM was loaded.
- Must continue to respect explicit ROM paths when provided by the user.
- Must not crash if the `homebrew.nes` file is missing (e.g., in a weird deployment); it should fall back to the existing "No ROM loaded" state gracefully.

## 🚫 Out of Scope
- A full graphical ROM browser UI on empty launch (Phase 2).
- Automatically downloading public domain ROMs from the internet.
