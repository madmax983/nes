# 🔭 Vantage: Spec for Bundled ROM Examples

* 👤 **User Story:** As a New User, I want the "Getting Started" examples in the README to run out-of-the-box, so that I don't hit "No such file or directory" errors on my first try.
* 💼 **Business Problem:** Every user who hits an immediate error running the README examples is a user who gives up on our emulator. We are losing adoption due to a brittle first experience caused by hardcoded, machine-specific paths (e.g. `C:\Users\markm\roms\...`).
* 🎯 **Success Metrics:** 100% of copy-pasted `cargo run` commands in the README work out-of-the-box on a fresh clone.
* ✅ **Acceptance Criteria:**
  - The README must use the bundled homebrew ROM (`./roms/homebrew/homebrew.nes`) in all quickstart launch commands instead of external `.nes` files.
  - Hardcoded local paths (like `C:\Users\markm\...`) must be entirely removed from the examples.
* 🚫 **Out of Scope:** Automating the downloading of commercial ROMs.