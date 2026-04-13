# 🔭 Vantage: Spec for ROM Hack Soft Patching

## 👤 User Story
"As a Retro Gamer, I want to play fan-made ROM hacks and translations without manually modifying my game files, so that I can easily try new community content without risking the corruption of my original clean ROMs."

## 💼 Business Problem (So What?)
**What business problem does this solve?**
The ROM hacking community (Kaizo Mario, randomizers, English translations) drives massive engagement in retro gaming. Currently, users must use clunky external third-party tools to patch their ROMs before loading them. By supporting native soft-patching, we remove significant friction, making our emulator the easiest way to consume user-generated content and retaining users who would otherwise seek out other feature-rich emulators.

## 📈 Success Metrics
- **Adoption:** 15% of ROM loads include an active patch file.
- **Engagement:** Increase in unique ROM hashes played per user, indicating easier exploration of hacks/randomizers.

## 🕵️ Gap Analysis
- **Market View:** Top-tier emulators (RetroArch, Mesen, Snes9x) support automatic soft-patching via `.ips` and `.bps` files.
- **Our Gap:** We currently only load `.nes` files. Users must manually pre-patch ROMs with external utilities, permanently altering their files or managing messy duplicate directories.

## ✅ Acceptance Criteria
- Must detect and auto-apply `.ips` and `.bps` patches if they share the same base filename as the loaded `.nes` ROM and reside in the same directory.
- Must apply the patch "softly" in memory during the ROM loading phase; the original `.nes` file on disk must never be modified.
- Must output a log message or UI indicator stating that a patch was successfully applied.
- Must gracefully fallback to loading the unpatched ROM if the patch file is corrupt or invalid.

## 🚫 Out of Scope
- Support for `.xdelta` or `.ups` patch formats (Phase 2).
- Built-in UI for creating or exporting patches from memory differences.
- Support for applying multiple patches to a single ROM simultaneously.
