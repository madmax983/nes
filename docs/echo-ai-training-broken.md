# 🗣️ Echo: AI Control Training example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the "AI Control Training" example from the README. The `prepare_smb_control` step worked fine, but when I ran the `train_smb_control` step, it crashed with: `Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The example configuration file `config/ai/profiles/smb-control.example.toml` hardcodes the ROM path to `./roms/Super Mario Bros.nes`, which doesn't exist by default. The README states it uses the bundled homebrew ROM for demonstration during the prepare step, but never mentions updating the config file to match.

* 💡 **The Fix:** Either update `smb-control.example.toml` to point to `./roms/homebrew/homebrew.nes` by default, or add an explicit step in the README instructing the user to edit the config file before running the training command.
