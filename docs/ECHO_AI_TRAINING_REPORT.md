# 🗣️ Echo: Getting Started with `nes-ai` is broken

## 🤦 The Confusion
I followed the exact steps in the `crates/nes-ai/README.md` to train a model. The instructions say to copy `config/ai/profiles/smb-control.example.toml` to `config/ai/profiles/smb-control.toml` and point `rom_path` / `snapshot_path` to local files. I did that, but when I ran the training command, it failed with:

`Failed to read profile config: No such file or directory (os error 2)`

When I created the config file with `cp config/ai/profiles/smb-control.example.toml config/ai/profiles/smb-control.toml` and tried again, it failed again:

`Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`

## 🕵️ The Reality
The `README.md` tells you to copy the example configuration file but doesn't explicitly mention that the default `rom_path` in `smb-control.example.toml` points to `./roms/Super Mario Bros.nes`, which doesn't exist unless you provide it.

To make it work, I had to manually edit the `config/ai/profiles/smb-control.toml` and change the `rom_path` to the bundled homebrew ROM `./roms/homebrew/homebrew.nes`.

The `README.md` in the root explicitly mentions:
`# 1. Prepare the fixed SMB 1-1 control snapshot (using the bundled homebrew ROM for demonstration)`

But the `crates/nes-ai/README.md` just assumes you know what to do. The user shouldn't have to manually edit an example config file just to get the basic `train_smb_control` demo working.

## 💡 The Fix
Update the `rom_path` in `config/ai/profiles/smb-control.example.toml` to point to `./roms/homebrew/homebrew.nes` by default, so it works out-of-the-box for demonstration purposes just like the `prepare_smb_control` command.
