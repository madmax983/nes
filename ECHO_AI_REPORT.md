# 🗣️ Echo: AI Training example fails due to missing ROM path

**Description:**

* 🤦 **The Confusion:** Followed the "AI Control Training" instructions in README. I successfully prepared the snapshot, copied the configuration, and ran the training command `cargo run -p nes-ai --bin train_smb_control ...`. It immediately failed with: `Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The example profile configuration (`config/ai/profiles/smb-control.example.toml`) hardcodes `rom_path = "./roms/Super Mario Bros.nes"`, which does not exist by default. However, the preparation step correctly points out to use the bundled homebrew ROM: `"./roms/homebrew/homebrew.nes"`. When a user directly copies the example configuration and runs the training command, it breaks because the ROM path in the config is still pointing to the missing Mario ROM instead of the homebrew ROM we just used to prepare the snapshot.

* 💡 **The Fix:** The `rom_path` in `config/ai/profiles/smb-control.example.toml` should be updated to point to the bundled `"./roms/homebrew/homebrew.nes"` by default so that the training example works out-of-the-box without requiring users to supply their own commercial ROM or manually edit the config file.
