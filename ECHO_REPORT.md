# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the desktop and netplay examples directly from the README block. The system immediately errored out with `Failed to read ROM at 'C:\Users\markm\roms\Super Mario Bros. (World).nes': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** Turns out the `README.md` examples use hardcoded local Windows paths pointing to a specific user's `markm` directory. As a new user, I don't have this directory, nor do I have these specific ROMs named exactly this way. The example just fails.

* 💡 **The Fix:** Change the quickstart commands in the README to point to the locally bundled homebrew ROM (`.\roms\homebrew\homebrew.nes`) or clearly indicate `<path-to-your-rom>.nes`. If I can't copy-paste and run it, I'm out!

# 🗣️ Echo: Getting Started config command fails

**Description:**

* 🤦 **The Confusion:** Copied and pasted `cargo run -p nes-tui -- --config ./nes.toml` from the "Desktop/TUI launch commands" section in the README. The application panicked right away with `failed to read config './nes.toml': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The project ships with `nes.example.toml` but not `nes.toml`. The README tells me to run a command against a file that doesn't exist out of the box unless I rename it myself.

* 💡 **The Fix:** Update the README instructions to explicitly say `cp nes.example.toml nes.toml` before running the commands, or change the default command in the README to `--config ./nes.example.toml`.
