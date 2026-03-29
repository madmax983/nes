# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the desktop and netplay examples directly from the README block. The system immediately errored out with `Failed to read ROM at 'C:\Users\markm\roms\Super Mario Bros. (World).nes': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** Turns out the `README.md` examples use hardcoded local Windows paths pointing to a specific user's `markm` directory. As a new user, I don't have this directory, nor do I have these specific ROMs named exactly this way. The example just fails.

* 💡 **The Fix:** Change the quickstart commands in the README to point to the locally bundled homebrew ROM (`.\roms\homebrew\homebrew.nes`) or clearly indicate `<path-to-your-rom>.nes`. If I can't copy-paste and run it, I'm out!

# 🗣️ Echo: V0 Implementation Plan commands fail for new users

**Description:**

* 🤦 **The Confusion:** Tried to run the verification commands in `docs/plans/2026-02-21-nes-v0-implementation-plan.md` like `C:\Users\markm\verus\verus.exe crates/nes-proof/src/cpu_model.rs`. Windows tells me the path cannot be found.
* 🕵️ **The Reality:** The plan has hardcoded paths pointing directly to `C:\Users\markm\verus\verus.exe`. I am not Mark! I installed verus via my package manager and it is in my system path.
* 💡 **The Fix:** Change `C:\Users\markm\verus\verus.exe` to a generic path like `verus` or `./verus.exe` in the implementation plan documentation so people can actually run the commands.
