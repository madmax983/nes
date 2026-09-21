# 🗣️ Echo: AI training example is broken

* 🤦 **The Confusion:** "Tried to run the `train_smb_control` command from the README. Got an error saying `./config/ai/profiles/smb-control.toml` does not exist."
* 🕵️ **The Reality:** "The `config/ai/profiles/` directory only contains `smb-control.example.toml`. The README jumps straight into training without telling the user to copy the config first."
* 💡 **The Fix:** "Add a command before step 2 in the README instructing the user to copy the example config: `cp config/ai/profiles/smb-control.example.toml config/ai/profiles/smb-control.toml`."
