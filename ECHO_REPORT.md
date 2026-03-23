# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the desktop and netplay examples directly from the README block. The system immediately errored out with `Failed to read ROM at 'C:\Users\markm\roms\Super Mario Bros. (World).nes': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** Turns out the `README.md` examples use hardcoded local Windows paths pointing to a specific user's `markm` directory. As a new user, I don't have this directory, nor do I have these specific ROMs named exactly this way. The example just fails.

* 💡 **The Fix:** Change the quickstart commands in the README to point to the locally bundled homebrew ROM (`.\roms\homebrew\homebrew.nes`) or clearly indicate `<path-to-your-rom>.nes`. If I can't copy-paste and run it, I'm out!

# 🗣️ Echo: Missing nes.toml configuration

**Description:**

* 🤦 **The Confusion:** Tried to run `cargo run -p nes-desktop --release -- --config ./nes.toml` and `cargo run -p nes-tui -- --config ./nes.toml` directly from the README block. The system immediately errored out with `failed to read config './nes.toml': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The repository doesn't include a `nes.toml` out of the box, only a `nes.example.toml`, but the README doesn't instruct the user to copy or rename it before running the commands.

* 💡 **The Fix:** Update the README to explicitly tell the user to run `cp nes.example.toml nes.toml` before running those commands, or change the commands to use `./nes.example.toml`.
