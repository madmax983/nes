# 🗣️ Echo: AI Control Training example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the AI Control Training example in the README. Step 1 (`prepare_smb_control`) worked, but Step 2 (`train_smb_control`) immediately failed with `Training failed: failed to read profile config: No such file or directory (os error 2)` when trying to read `./config/ai/profiles/smb-control.toml`.

* 🕵️ **The Reality:**
  1. The repo doesn't contain `./config/ai/profiles/smb-control.toml` out of the box, it only contains `smb-control.example.toml`.
  2. Even if a user guesses they need to copy it (like with `nes.toml`), the training step *still* fails! The example config hardcodes `rom_path = "./roms/Super Mario Bros.nes"`, but Step 1 instructs the user to prepare the snapshot using `"./roms/homebrew/homebrew.nes"`! The training step then fails with `failed to read ROM './roms/Super Mario Bros.nes'`.

* 💡 **The Fix:**
  Update the README instructions to explicitly include a command to copy the AI config: `cp ./config/ai/profiles/smb-control.example.toml ./config/ai/profiles/smb-control.toml`. Also, update the `smb-control.example.toml` file so its `rom_path` points to `"./roms/homebrew/homebrew.nes"` so the example can actually run out of the box as promised.