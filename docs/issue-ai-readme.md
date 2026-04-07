# 🗣️ Echo: Getting Started example is broken

🤦 **The Confusion:** Tried to run the training example from `crates/nes-ai/README.md`. It failed with `Error: Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`.

🕵️ **The Reality:** The `README.md` tells me to copy `config/ai/profiles/smb-control.example.toml` to `config/ai/profiles/smb-control.toml` and "point `rom_path` / `snapshot_path` at your local files". However, the example `.toml` file hardcodes the path to `./roms/Super Mario Bros.nes`, which doesn't exist by default. If I just run the example as written in the README, it fails.

💡 **The Fix:** Either ship the repo with a placeholder ROM at `./roms/Super Mario Bros.nes`, or better yet, change the default in the example `.toml` to `./roms/homebrew/homebrew.nes` which actually exists in the repo, so the copy-paste just works.
