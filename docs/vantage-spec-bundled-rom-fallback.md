# 🔭 Vantage: Spec for Bundled ROM Fallback

## 👤 **User Story**
As a new developer or user exploring the project, I want the emulator to automatically load a bundled homebrew ROM if I do not provide a valid ROM path, so that I can immediately see the system working without needing to source external ROM files.

## 💼 **Business Problem (So What?)**
The current onboarding experience is broken. The `README.md` and default run commands assume the existence of local ROM files at specific absolute paths. When these fail, the user is presented with a hard crash (`os error 2`). This creates friction, increases time-to-first-value (TTFV), and causes users to abandon the project before experiencing its capabilities. Complexity in setup is a cost; immediate utility is a revenue.

## 📈 **Success Metrics**
- **Time to First Play:** < 10 seconds for a new user executing `cargo run -p nes-desktop`.
- **Onboarding Success Rate:** 100% of new clones can launch the emulator successfully without manual configuration or downloading external files.

## ✅ **Acceptance Criteria**
- When a user launches the emulator without specifying a ROM path, the system must automatically load `./roms/homebrew/homebrew.nes`.
- When a user specifies a ROM path that does not exist, the system must log a warning and fall back to loading the bundled homebrew ROM.
- The fallback behavior must be clearly communicated to the user via console output (e.g., "Warning: ROM not found. Falling back to bundled homebrew ROM.").
- The bundled homebrew ROM must be included in the repository.

## 🚫 **Out of Scope**
- Downloading ROMs from the internet dynamically.
- Providing multiple bundled ROMs or a ROM selection menu.
- Modifying the web version's ROM loading behavior (this focuses on desktop/terminal native onboarding).