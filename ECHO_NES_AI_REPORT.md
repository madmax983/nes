# 🗣️ Echo: AI Training example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the `train_smb_control` example from the `nes-ai` README. It failed with `Failed to read profile config: No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The README tells me in a paragraph of text to copy `config/ai/profiles/smb-control.example.toml` to `config/ai/profiles/smb-control.toml`, but there is no executable command block for it. As an impatient user, I just skipped to the code block.

* 💡 **The Fix:** Add an explicit `cp config/ai/profiles/smb-control.example.toml config/ai/profiles/smb-control.toml` command block *before* the `train_smb_control` command block so users can just copy-paste.
