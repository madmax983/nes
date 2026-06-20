# 🗣️ Echo: Getting Started with nes-tui requires manual config copy

**Description:**

* 🤦 **The Confusion:** Tried to run the `nes-tui` app out-of-the-box using the example `cargo run -p nes-tui -- --config ./nes.toml`. It instantly failed because `nes.toml` did not exist.
* 🕵️ **The Reality:** The repository provides a `nes.example.toml` file, but the user is expected to manually copy it over to `nes.toml` before the run commands work.
* 💡 **The Fix:** The application (like `nes-tui` or `nes-config`) should gracefully handle the missing config by falling back to the `nes.example.toml` or clearly prompt the user.
