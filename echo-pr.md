🤦 **The Confusion:**
Tried to launch the emulator with `cargo run -p nes-desktop --release -- ./roms/homebrew/homebrew.nes` from the "Desktop/TUI launch commands" section. It failed with `Error: Could not find the ROM file at './roms/homebrew/homebrew.nes'`.

🕵️ **The Reality:**
Turns out the bundled homebrew ROM isn't checked into the repo. It has to be built first using `cargo run -p nes-test-harness --bin build_homebrew_rom`. But that command is hidden way at the bottom of the README in the "Homebrew ROM" section, *after* the launch instructions!

💡 **The Fix:**
Move the homebrew build command up to the "Desktop/TUI launch commands" section so users can actually run the first example, or add a bold note pointing them to it before telling them to launch.
