# 🗣️ Echo: AI Training example is broken

🤦 **The Confusion:**
I followed the "AI Control Training" instructions in the `README.md` and tried to run the training command:
`cargo run -p nes-ai --bin train_smb_control -- ./config/ai/profiles/smb-control.toml 4 ./artifacts/ai/checkpoints ./artifacts/ai/eval`

It immediately crashed with the error:
`Failed to read profile config: No such file or directory (os error 2)`
What file is it missing?! It just says "No such file".

🕵️ **The Reality:**
The file `./config/ai/profiles/smb-control.toml` does not exist in the repository! There is only a `smb-control.example.toml` file in that directory. The README never tells me to copy or create the actual file before running the command. In addition, the error message doesn't print the file path it failed to read, making it hard to debug.

💡 **The Fix:**
1. Update the README to include a step for copying the example configuration before running the training command: `cp ./config/ai/profiles/smb-control.example.toml ./config/ai/profiles/smb-control.toml`.
2. Improve the error message in the code to actually print the file path it tried to read, so it says "Failed to read profile config at ./config/ai/profiles/smb-control.toml: No such file or directory (os error 2)".
