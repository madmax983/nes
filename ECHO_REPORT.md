# 🗣️ Echo: Getting Started commands fail due to missing .toml files

**Description:**

* 🤦 **The Confusion:** I tried to run the commands in the README such as `cargo run -p nes-desktop --release -- --config ./nes.toml` and noticed that the file does not exist.

* 🕵️ **The Reality:** The repository only contains template files like `nes.example.toml`, `smb-any.example.toml`, and `smb-control.example.toml`. The actual `.toml` files expected by the commands don't exist, and the README never tells the user they need to manually copy and rename these files before running the commands.

* 💡 **The Fix:** Add a clear instruction in the README before the launch commands telling the user to copy the `.example.toml` files to `.toml` files (e.g. `cp nes.example.toml nes.toml`).
