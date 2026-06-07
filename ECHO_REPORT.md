# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to follow the "Training" instructions in `crates/nes-ai/README.md`. I copied `config/ai/profiles/smb-control.example.toml` as instructed, then ran the training command: `cargo run -p nes-ai --bin train_smb_control -- ./config/ai/profiles/smb-control.toml 4 ./artifacts/ai/checkpoints ./artifacts/ai/eval`. It crashed with `Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory`.

* 🕵️ **The Reality:** The example `smb-control.example.toml` hardcodes the `rom_path` to `"./roms/Super Mario Bros.nes"`, which is a commercial ROM that doesn't exist in the repo. The README does technically say "point `rom_path` / `snapshot_path` at your local files" in a previous paragraph, but the example commands just blindly tell you to run it without explicitly editing the `.toml` to point to the bundled `homebrew.nes`.

* 💡 **The Fix:** Update `config/ai/profiles/smb-control.example.toml` to default to the bundled homebrew ROM (`"./roms/homebrew/homebrew.nes"`) so the copy-paste run path "just works", or add an explicit `sed` or manual editing step in the README right before the `cargo run` command.
