# 🗣️ Echo: Getting Started examples are broken and confusing

**Description:**

* 🤦‍♂️ **The Confusion:** I tried to run the "Desktop/TUI launch commands" straight from the README. The second command `cargo run -p nes-desktop --release -- --config ./nes.toml` immediately crashed with `failed to read config './nes.toml': No such file or directory (os error 2)`. Then I tried the "RTA mode" auto-select command and it crashed with `Failed to enter RTA mode... No RTA profile matched ROM hash`. Also, why are all the scripts using `powershell`? I'm on Linux/macOS and none of this works!

* 🕵️‍♂️ **The Reality:** Turns out `nes.toml` doesn't exist out of the box; there's only a `nes.example.toml` that I apparently have to copy and rename first. For the RTA mode, the auto-select command uses `homebrew.nes` but the only included profile is `smb-any`, which has a completely different hash. And the README assumes everyone is on Windows using PowerShell, ignoring Linux and macOS users entirely.

* 💡 **The Fix:**
  1. Add a step in the README telling users to copy `nes.example.toml` to `nes.toml` before running the config example, or change the command to use the example file.
  2. Provide a default RTA profile that actually matches the bundled `homebrew.nes` so the auto-select command works out of the box.
  3. Provide `sh` or `bash` alternatives for the automation scripts, or at least mention that PowerShell is required. If I can't copy-paste and run it, I am leaving!
