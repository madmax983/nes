# 🔭 Vantage: Spec for Glitch Mode (ROM Corruptor)

## 👤 User Story
"As a Content Creator or Chaos Gamer, I want a 'Glitch Mode' that deterministically corrupts the ROM data before loading, so that I can experience hilarious, unexpected gameplay scenarios, broken graphics, and unpredictable behavior for entertainment purposes."

## ❓ So What? (Business Problem)
**What business problem does this solve?**
"ROM Corruptors" have a huge niche audience on platforms like YouTube and Twitch, where creators play intentionally broken versions of classic games. Currently, users must use external third-party software to corrupt a ROM file, save it, and load it into an emulator. By integrating the existing `RomCorruptor` directly into our launch options, we capture this specific creator market, making our emulator the easiest "one-click chaos" solution available. It drives viral shareability and community engagement.

## 📊 Success Metrics
- **Reliability:** The same seed and intensity always produces the exact same corruption pattern (deterministic).
- **Utility:** The emulator gracefully handles crashes or invalid opcodes caused by the corruption without taking down the host OS or UI.
- **Adoption:** Dedicated streams or videos feature the emulator specifically for its built-in Glitch Mode.

## 🕵️ Gap Analysis
- **Market View:** There are standalone tools (like the Real-Time Corruptor or old flash-based tools) dedicated to destroying ROMs, but very few modern, accurate emulators build this in as a native, easily configurable feature.
- **Our Gap:** We have the deterministic `RomCorruptor` logic in `nes-core`, but no CLI flags, UI toggles, or safety rails in `nes-desktop` to let users safely invoke it on load.

## ✅ Acceptance Criteria
- Must provide a CLI flag (e.g., `--glitch-seed <num> --glitch-intensity <0-100>`) to invoke the ROM corruptor on startup.
- Must provide a UI element in the settings to enable Glitch Mode and set the seed/intensity for the next ROM load.
- Must apply the corruption *in memory only*, never modifying the original ROM file on disk.
- Must allow selecting whether to corrupt PRG (logic), CHR (graphics), or both.
- Must ensure that if the corrupted logic causes a CPU crash (e.g., KIL opcodes), the emulator UI remains responsive and allows resetting the game.

## 🚫 Out of Scope
- Real-time memory corruption during gameplay (Phase 1 is load-time ROM corruption only).
- Advanced corruption algorithms (e.g., specific instruction targeting); we will use the existing LCG byte/bit scrambling for now.
