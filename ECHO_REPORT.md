# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run `cargo run -p nes-desktop --release -- --config ./nes.toml` from the "Desktop/TUI launch commands" section. Got an error: `failed to read config './nes.toml': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The workspace doesn't have a `nes.toml` by default, it has a `nes.example.toml`. The README says "Runtime and ROM paths are configured through `nes.toml` at the workspace root", but never instructs the user to create or copy the file before running the command.

* 💡 **The Fix:** Add a step before the launch commands instructing users to copy the example config: `cp nes.example.toml nes.toml`.

# 🗣️ Echo: Getting Started example is broken (AI Control Training)

**Description:**

* 🤦 **The Confusion:** Tried to run `cargo run -p nes-ai --bin train_smb_control -- ...` from the "AI Control Training" section. Got an error: `Failed to read profile config: No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The workspace doesn't have a `smb-control.toml` by default, it has a `smb-control.example.toml`. The README says to "Train from the local profile", but doesn't explicitly instruct the user to copy the example configuration first.

* 💡 **The Fix:** Add a step before the launch command instructing users to copy the example config: `cp ./config/ai/profiles/smb-control.example.toml ./config/ai/profiles/smb-control.toml`. Also, add a note to edit the config to use a valid `rom_path` (e.g. `./roms/homebrew/homebrew.nes`).
