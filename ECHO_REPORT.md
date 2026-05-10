# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run `cargo run -p nes-desktop --release -- --config ./nes.toml` from the "Desktop/TUI launch commands" section. Got an error: `failed to read config './nes.toml': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The workspace doesn't have a `nes.toml` by default, it has a `nes.example.toml`. The README says "Runtime and ROM paths are configured through `nes.toml` at the workspace root", but never instructs the user to create or copy the file before running the command.

* 💡 **The Fix:** Add a step before the launch commands instructing users to copy the example config: `cp nes.example.toml nes.toml`.

# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the `story_demo`. Compiler said `NarrativeGenerator` not found.

* 🕵️ **The Reality:** Turns out I needed to enable feature `nova`.

* 💡 **The Fix:** Add a huge banner in README saying 'REQUIRES FEATURE NOVA'.
