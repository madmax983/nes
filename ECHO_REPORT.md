# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the desktop and tui examples directly from the README block. The system immediately errored out with `failed to read config './nes.toml': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** Turns out the `README.md` examples tell you to run `cargo run -p nes-desktop --release -- --config ./nes.toml`. But there is no `nes.toml` file in the repository! There is only a `nes.example.toml`. As a new user, I didn't know I had to copy the example file first because the README never mentions it! The example just fails.

* 💡 **The Fix:** Add a step to the README to copy `nes.example.toml` to `nes.toml` before running the command, or change the quickstart commands to use `./nes.example.toml`. If I can't copy-paste and run it, I'm out!
