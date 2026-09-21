# 🗣️ Echo: AI Training Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the AI training command from `crates/nes-ai/README.md` (`cargo run -p nes-ai --bin train_smb_control ...`). Got an error: `Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory (os error 2)`.
* 🕵️ **The Reality:** The example config `config/ai/profiles/smb-control.example.toml` hardcodes a non-existent ROM path. The README mentions updating it, but I just copy-pasted the commands as a new user.
* 💡 **The Fix:** Update `smb-control.example.toml` to point to the bundled `./roms/homebrew/homebrew.nes` by default so the copy-paste example works out of the box.
