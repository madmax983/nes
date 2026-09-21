# 🗣️ Echo: nes-ai Training example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the `train_smb_control` step from the `nes-ai/README.md` after preparing the snapshot with the bundled homebrew ROM. Got an error: `Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`.
* 🕵️ **The Reality:** The README tells you to use `homebrew.nes` for the snapshot preparation, and then to copy `smb-control.example.toml` for the training profile. But that example config hardcodes `rom_path = "./roms/Super Mario Bros.nes"`, which doesn't exist by default.
* 💡 **The Fix:** Update `smb-control.example.toml` to point to `./roms/homebrew/homebrew.nes` so the out-of-the-box example works without hunting for commercial ROMs.
