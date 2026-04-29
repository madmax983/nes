# 🗣️ Echo: AI Control Training example is broken

🤦 **The Confusion:**
I copy-pasted the command from the README to run the `train_smb_control` AI training step:
```powershell
cargo run -p nes-ai --bin train_smb_control -- `
  ./config/ai/profiles/smb-control.toml `
  4 `
  ./artifacts/ai/checkpoints `
  ./artifacts/ai/eval
```

But it immediately crashed with:
`Failed to read profile config: No such file or directory (os error 2)`

🕵️ **The Reality:**
Turns out `./config/ai/profiles/smb-control.toml` doesn't exist out of the box! There is only a `smb-control.example.toml` file in that directory. The README never told me I needed to copy it.

💡 **The Fix:**
Update the README to explicitly include a step to copy the example config first, like:
```powershell
cp ./config/ai/profiles/smb-control.example.toml ./config/ai/profiles/smb-control.toml
```
Or even better, just make the example command use the `.example.toml` file directly if we're just demoing it!
