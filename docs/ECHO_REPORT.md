# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the `train_smb_control` AI Control Training example. The first step (`prepare_smb_control`) worked since it explicitly uses the homebrew ROM, but the second step (`train_smb_control`) failed with: `Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`.
* 🕵️ **The Reality:** The training example instructs users to copy `config/ai/profiles/smb-control.example.toml`, but that default profile hardcodes the `rom_path` to a commercial ROM (`"./roms/Super Mario Bros.nes"`) instead of the bundled `./roms/homebrew/homebrew.nes`.
* 💡 **The Fix:** Update `config/ai/profiles/smb-control.example.toml` to point to the bundled `"./roms/homebrew/homebrew.nes"` by default, or add a huge warning in the README to edit the config file before running the training command.
