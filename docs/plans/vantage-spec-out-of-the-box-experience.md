# 🔭 Vantage: Spec for Out-of-the-box Experience

👤 **User Story:**
As a new user trying the emulator, I want it to load a bundled homebrew ROM or show a helpful 'Welcome' screen out-of-the-box, so that I can see it working immediately without needing to supply my own commercial ROMs.

💼 **Business Problem (So What?):**
The README quickstart often causes errors because users lack the specific ROMs required, increasing the friction to adoption and giving the impression that the software is broken.

📈 **Success Metrics:**
Zero configuration needed to see a running emulator window after `cargo run -p nes-desktop`.

🔍 **Gap Analysis:**
Currently, `cargo run -p nes-desktop` without a ROM argument tries to load a non-existent ROM or fails, whereas competitors like RetroArch or FCEUX either start with a blank UI or provide a default interface.

✅ **Acceptance Criteria:**
- Running `cargo run -p nes-desktop` with no arguments loads a default bundled ROM (e.g., `./roms/homebrew/homebrew.nes`).
- If the default bundled ROM is missing, it should display a clear, friendly error message instead of a generic OS error 2.

🚫 **Out of Scope:**
Downloading commercial ROMs automatically; building a full ROM library manager.
