# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the desktop and netplay examples directly from the README block. The system immediately errored out with `Failed to read ROM at 'C:\Users\markm\roms\Super Mario Bros. (World).nes': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** Turns out the `README.md` examples use hardcoded local Windows paths pointing to a specific user's `markm` directory. As a new user, I don't have this directory, nor do I have these specific ROMs named exactly this way. The example just fails.

* 💡 **The Fix:** Change the quickstart commands in the README to point to the locally bundled homebrew ROM (`.\roms\homebrew\homebrew.nes`) or clearly indicate `<path-to-your-rom>.nes`. If I can't copy-paste and run it, I'm out!

---

# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run `cargo run -p nes-tui -- --config ./nes.toml` as instructed in README. System failed with `failed to read config './nes.toml': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** Turns out the repo provides `nes.example.toml` instead of `nes.toml`, so the quickstart command fails out-of-the-box unless I rename it first.

* 💡 **The Fix:** Change the quickstart command in the README to explicitly tell users to copy `nes.example.toml` to `nes.toml` first, or change the default command to use the example config.
