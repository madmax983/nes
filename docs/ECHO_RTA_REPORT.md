# 🗣️ Echo: RTA mode examples are broken

🤦 **The Confusion:**
Tried to run the `cargo run -p nes-desktop --release -- --rta --rta-calibrate --rta-profile smb-any --rta-profiles-dir ./config/rta/profiles ./roms/homebrew/homebrew.nes` from the README, but it crashed.

🕵️ **The Reality:**
I noticed that it failed because the `smb-any.toml` profile does not exist. There is a `smb-any.example.toml` in that directory. The README instructs to use `--rta-profile smb-any` but never tells the user to set up the configuration.

💡 **The Fix:**
Add an instruction to copy the example configuration first, e.g., `cp ./config/rta/profiles/smb-any.example.toml ./config/rta/profiles/smb-any.toml`.
