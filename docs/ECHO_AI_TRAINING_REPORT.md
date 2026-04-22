# 🗣️ Echo: AI Control Training examples are broken

**Description:**

* 🤦 **The Confusion:** Tried to run the training example from the main README (`cargo run -p nes-ai --bin train_smb_control -- ./config/ai/profiles/smb-control.toml 4 ./artifacts/ai/checkpoints ./artifacts/ai/eval`). It crashed immediately with an unhelpful error: `Failed to read profile config: No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The file `./config/ai/profiles/smb-control.toml` does not exist in the repository. Only `./config/ai/profiles/smb-control.example.toml` exists. The main README instructs users to run the train command but forgets to tell them to copy the example config first (unlike the desktop launcher example which does mention it). The `nes-ai/README.md` mentions copying it, but the main README does not, and the error message just gives an obscure OS error.

* 💡 **The Fix:**
1. In the main `README.md`, add a step to copy the example config file (`cp ./config/ai/profiles/smb-control.example.toml ./config/ai/profiles/smb-control.toml`) before the training command.
2. Make the error message helpful (e.g. "Config file not found at path. Did you forget to copy the .example.toml?").
