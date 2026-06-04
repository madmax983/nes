# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run `cargo run -p nes-desktop --release -- --config ./nes.toml` from the "Desktop/TUI launch commands" section. Got an error: `failed to read config './nes.toml': file not found. Hint: copy the example profile (e.g. cp nes.example.toml nes.toml)`.

* 🕵️ **The Reality:** The workspace doesn't have a `nes.toml` by default, it has a `nes.example.toml`. The README says "First, copy the example configuration... cp nes.example.toml nes.toml", but the actual execution command requires it and fails if I skipped that line.

* 💡 **The Fix:** Add a huge banner in README saying 'REQUIRES COPYING nes.example.toml to nes.toml' before the run commands.
