# 🗣️ Echo: nes-ai docs assume files exist

**Description:**

* 🤦 **The Confusion:** Tried to follow the "AI Control Training" section in README.md. First command `cargo run -p nes-ai --bin prepare_smb_control -- ...` passed. Second command `cargo run -p nes-ai --bin train_smb_control -- ./config/ai/profiles/smb-control.toml ...` failed with `Failed to read profile config: No such file or directory (os error 2)`.

* 🕵️ **The Reality:** I looked in `./config/ai/profiles/` and there is no `smb-control.toml`, only a `smb-control.example.toml`. Even if I copy it over, it expects a ROM at `./roms/Super Mario Bros.nes` which I don't have, leading to `failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`.

* 💡 **The Fix:** The docs should tell the user to copy the example profile, and update the profile file to use `./roms/homebrew/homebrew.nes` instead of `Super Mario Bros.nes` so that it actually runs out of the box with the bundled homebrew ROM.
