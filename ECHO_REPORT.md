# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run `cargo run -p nes-desktop --release -- --config ./nes.toml` from the "Desktop/TUI launch commands" section. Got an error: `failed to read config './nes.toml': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The workspace doesn't have a `nes.toml` by default, it has a `nes.example.toml`. The README says "Runtime and ROM paths are configured through `nes.toml` at the workspace root", but never instructs the user to create or copy the file before running the command.

* 💡 **The Fix:** Add a step before the launch commands instructing users to copy the example config: `cp nes.example.toml nes.toml`.

# 🗣️ Echo: AI Control Training example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the AI training example `cargo run -p nes-ai --bin train_smb_control -- ./config/ai/profiles/smb-control.toml 4 ./artifacts/ai/checkpoints ./artifacts/ai/eval` from the README. It immediately failed with: `Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The default AI profile configuration in `nes-ai` (`config/ai/profiles/smb-control.example.toml`) hardcodes a non-existent ROM path (`./roms/Super Mario Bros.nes`). This file is not provided in the repository, so the user has to either provide their own ROM and update the config, or use a bundled ROM (like the homebrew one). The README provides no instructions to update the config after copying it.

* 💡 **The Fix:** Add a huge banner in the README saying 'REQUIRES VALID ROM IN CONFIG' or update the example config to use the bundled homebrew ROM by default (`./roms/homebrew/homebrew.nes`).
