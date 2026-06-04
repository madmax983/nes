# 🗣️ Echo: AI Training example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run `cargo run -p nes-ai --bin train_smb_control -- ./config/ai/profiles/smb-control.toml 4 ./artifacts/ai/checkpoints ./artifacts/ai/eval` from the "AI Control Training" section in the README. Got an error: `Failed to read profile config: No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The repository doesn't have a `smb-control.toml` file in `config/ai/profiles/` by default. It has a `smb-control.example.toml` file. The README instructs the user to run the training command, but skips telling the user they need to copy the example profile first.

* 💡 **The Fix:** Add a step before the `train_smb_control` command instructing users to copy the example AI config: `cp config/ai/profiles/smb-control.example.toml config/ai/profiles/smb-control.toml`.
