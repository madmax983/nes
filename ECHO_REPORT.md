# 🗣️ Echo: AI training example is broken

**Description:**

* 🤦 **The Confusion:** Tried to follow the "AI Control Training" steps in the README to train an SMB control profile. The `prepare_smb_control` step worked because it explicitly pointed to `./roms/homebrew/homebrew.nes`. However, running `train_smb_control` with the provided example config failed with an error: `Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`.
* 🕵️ **The Reality:** The `config/ai/profiles/smb-control.example.toml` file hardcodes `rom_path = "./roms/Super Mario Bros.nes"`, which doesn't exist by default. The example commands don't explain that the config needs to be manually edited to point to the homebrew ROM to complete the tutorial.
* 💡 **The Fix:** Either change the example config to point to `./roms/homebrew/homebrew.nes` by default, or add a step in the README explicitly telling the user to edit the copied `smb-control.toml` to fix the ROM path.
