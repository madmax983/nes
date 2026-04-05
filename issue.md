# 🗣️ Echo: Getting Started examples fail due to missing nes.toml

🤦 **The Confusion:**
I tried to run the "Desktop/TUI launch commands" from the README (`cargo run -p nes-desktop --release -- --config ./nes.toml` and `cargo run -p nes-tui -- --config ./nes.toml`), but both crashed with a nasty error: `failed to read config './nes.toml': No such file or directory (os error 2)`.

🕵️ **The Reality:**
Turns out there is no `nes.toml` by default! The README mentions `nes.example.toml` in passing for netplay, but never tells me I need to copy it to `nes.toml` before running the basic launch commands.

💡 **The Fix:**
Add a direct instruction to the README before the run commands telling users to run `cp nes.example.toml nes.toml`.
