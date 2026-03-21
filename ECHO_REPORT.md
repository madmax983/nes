# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the desktop and netplay examples directly from the README block. The system immediately errored out with `Failed to read ROM at 'C:\Users\markm\roms\Super Mario Bros. (World).nes': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** Turns out the `README.md` examples use hardcoded local Windows paths pointing to a specific user's `markm` directory. As a new user, I don't have this directory, nor do I have these specific ROMs named exactly this way. The example just fails.

* 💡 **The Fix:** Change the quickstart commands in the README to point to the locally bundled homebrew ROM (`.\roms\homebrew\homebrew.nes`) or clearly indicate `<path-to-your-rom>.nes`. If I can't copy-paste and run it, I'm out!

# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the `story_demo`. Compiler said `NarrativeGenerator` not found.

* 🕵️ **The Reality:** Turns out I needed to enable feature `nova`.

* 💡 **The Fix:** Add a huge banner in README saying 'REQUIRES FEATURE NOVA'.
