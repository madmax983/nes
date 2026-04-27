# 🗣️ Echo: AI Training example is broken

🤦 **The Confusion:**
Tried to run the `train_smb_control` example from the README "AI Control Training" section. It failed with `Failed to read profile config: No such file or directory (os error 2)`.

🕵️ **The Reality:**
The workspace doesn't have a `config/ai/profiles/smb-control.toml` by default, only a `smb-control.example.toml`. The documentation tells the user to run the train command pointing to `smb-control.toml`, but never provides instructions to create or copy it from the example.

💡 **The Fix:**
Add a step before the AI commands instructing users to copy the example config: `cp ./config/ai/profiles/smb-control.example.toml ./config/ai/profiles/smb-control.toml`.
