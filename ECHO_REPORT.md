# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the TUI example directly from the README block. The system immediately errored out with `failed to read config './nes.toml': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** Turns out the `README.md` example commands explicitly pass `--config ./nes.toml`, but that file doesn't exist in a fresh clone! There is a `nes.example.toml` in the repo, but the README never tells me I need to copy or rename it before running the examples. As a new user, I just copy-paste and it fails immediately.

* 💡 **The Fix:** Add a step *before* the run commands in the `README.md` that explicitly says "Copy the example config: `cp nes.example.toml nes.toml`" so the commands actually work out of the box. If I have to guess how to setup the config file, the documentation failed.
