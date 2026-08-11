# 🗣️ Echo: AI Training example fails on missing ROM

**Description:**

* 🤦 **The Confusion:** Followed the README instructions to copy `config/ai/profiles/smb-control.example.toml` to `config/ai/profiles/smb-control.toml` and run `cargo run -p nes-ai --bin train_smb_control ...`. The command failed with `failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The default AI profile configuration in `smb-control.example.toml` hardcodes `rom_path = "./roms/Super Mario Bros.nes"`, which doesn't exist out-of-the-box. The `prepare_smb_control` step right above it uses `./roms/homebrew/homebrew.nes`.

* 💡 **The Fix:** Change `rom_path` in `config/ai/profiles/smb-control.example.toml` to `"./roms/homebrew/homebrew.nes"` so it matches the other examples and runs correctly out of the box.
