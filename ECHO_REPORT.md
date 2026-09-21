# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Followed the README for "AI Control Training". Step 1 told me to use `homebrew.nes` and it worked perfectly. But when I ran Step 2 (`train_smb_control`), the program crashed with `failed to read ROM './roms/Super Mario Bros.nes': No such file or directory`.
* 🕵️ **The Reality:** The `smb-control.example.toml` configuration file that the README tells you to copy has `rom_path = "./roms/Super Mario Bros.nes"` hardcoded in it. Since I was instructed to use `homebrew.nes` for the demo, I obviously didn't have the commercial ROM sitting there, so the example fails out-of-the-box.
* 💡 **The Fix:** We should change the default `rom_path` in `config/ai/profiles/smb-control.example.toml` to `"./roms/homebrew/homebrew.nes"` so the example can run perfectly from start to finish without requiring any manual file edits.
