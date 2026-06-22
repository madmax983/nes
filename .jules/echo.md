# 🗣️ Echo: Getting Started example is broken for nes-ai

🤦 **The Confusion:**
Tried to run the `train_smb_control` from `crates/nes-ai/README.md`. It told me to "copy `config/ai/profiles/smb-control.example.toml` to `config/ai/profiles/smb-control.toml`". But when I looked inside `crates/nes-ai/config/ai/profiles/`, the directory didn't exist! Then I looked at the code snippet, and I noticed that if I try to use the library directly to make a configuration, I have to import 4 different structs (`AiProfileConfig`, `GameProfileId`, `ObservationConfig`, `RewardConfig`) just to create one config.

🕵️ **The Reality:**
Turns out the `config` folder is at the workspace root, not inside `crates/nes-ai/`. And for the API, `AiProfileConfig` requires all these nested structs because of the strict separation of concerns, which makes it super tedious to use without the TOML file.

💡 **The Fix:**
Update the README to clearly state that the `config` folder is in the workspace root (e.g. `../../config/...`). And please, add a `Default` implementation or a builder pattern for `AiProfileConfig` so I don't have to import half the library just to spin up an environment!
