# 🗣️ Echo: Getting Started example is broken

🤦 **The Confusion:**
"Tried to run the AI Control Training example for `train_smb_control` and `eval_smb_control` from the README. The terminal said `Failed to read profile config: No such file or directory (os error 2)`."

🕵️ **The Reality:**
"Turns out the README tells me to use `./config/ai/profiles/smb-control.toml`, but that file doesn't exist out of the box! There is only a `./config/ai/profiles/smb-control.example.toml`."

💡 **The Fix:**
"Add a step in the README right before training to copy the example config, like `cp ./config/ai/profiles/smb-control.example.toml ./config/ai/profiles/smb-control.toml`, similar to what is done for `nes.example.toml`. And maybe also mention users need to make sure the rom path in it is correct!"
