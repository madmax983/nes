# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the examples directly from the README block. The basic configuration example (`cargo run -p nes-desktop --release -- --config ./nes.toml`) fails immediately with `Error: failed to read config './nes.toml': No such file or directory`. The RTA mode example (`--rta-profile smb-any`) also fails. The AI training example (`--bin train_smb_control`) crashes with `Failed to read profile config: No such file or directory`.

* 🕵️ **The Reality:** Turns out the `README.md` examples assume several `.toml` configuration files and profiles (`nes.toml`, `smb-any.toml`, `smb-control.toml`) exist in the workspace, but only `.example.toml` versions are checked into the repository. As a new user, I shouldn't have to guess that I need to manually copy these files before the provided examples will run.

* 💡 **The Fix:** Either change the README examples to point directly to the `.example.toml` files, or explicitly tell the user to run `cp nes.example.toml nes.toml`, `cp ./config/rta/profiles/smb-any.example.toml ./config/rta/profiles/smb-any.toml`, etc., before running the commands.