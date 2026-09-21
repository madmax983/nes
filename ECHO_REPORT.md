# 🗣️ Echo: AI Training profile config error message is unhelpful

**Description:**

* 🤦 **The Confusion:** Tried to run `cargo run -p nes-ai --bin train_smb_control -- ./config/ai/profiles/smb-control.toml 4 ./artifacts/ai/checkpoints ./artifacts/ai/eval`. Got an error: `Failed to read profile config: No such file or directory (os error 2)`.

* 🕵️ **The Reality:** I forgot to copy `config/ai/profiles/smb-control.example.toml` to `config/ai/profiles/smb-control.toml`. The error message just gave me a generic `os error 2` instead of telling me what file it was looking for or how to fix it.

* 💡 **The Fix:** Make the error message helpful. It should print the file path it tried to read, and ideally say `Hint: copy the example profile (e.g. cp config/ai/profiles/smb-control.example.toml config/ai/profiles/smb-control.toml)` just like the main `nes-config` loader does!
