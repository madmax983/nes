# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the `train_smb_control` example from the "AI Control Training" section. Got an error: `failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** Turns out `smb-control.example.toml` defaults to pointing at the real Super Mario Bros ROM, but the README step just says to copy it and start training without mentioning I needed to edit it to point to the `homebrew.nes` ROM that the previous step generated a snapshot for.

* 💡 **The Fix:** Either tell users to edit `smb-control.toml` to change the `rom_path` to `./roms/homebrew/homebrew.nes`, or update the `smb-control.example.toml` to use the homebrew ROM by default.
