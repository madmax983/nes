# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the desktop and netplay examples directly from the README block. The system immediately errored out with `Failed to read ROM at 'C:\Users\markm\roms\Super Mario Bros. (World).nes': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** Turns out the `README.md` examples use hardcoded local Windows paths pointing to a specific user's `markm` directory. As a new user, I don't have this directory, nor do I have these specific ROMs named exactly this way. The example just fails.

* 💡 **The Fix:** Change the quickstart commands in the README to point to the locally bundled homebrew ROM (`.\roms\homebrew\homebrew.nes`) or clearly indicate `<path-to-your-rom>.nes`. If I can't copy-paste and run it, I'm out!

---

# 🗣️ Echo: Missing nes.toml configuration file out of the box

**Description:**

* 🤦 **The Confusion:** Tried to run the TUI example directly from the README block using `cargo run -p nes-tui -- --config ./nes.toml`. The system immediately errored out with `failed to read config './nes.toml': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The `README.md` example commands explicitly pass `--config ./nes.toml`, but that file doesn't exist in a fresh clone! There is a `nes.example.toml` in the repo, but the README never tells me I need to copy or rename it before running the examples. As a new user, I just copy-paste and it fails immediately.

* 💡 **The Fix:** Add a step *before* the run commands in the `README.md` that explicitly says "Copy the example config: `cp nes.example.toml nes.toml`" so the commands actually work out of the box. If I have to guess how to setup the config file, the documentation failed.
