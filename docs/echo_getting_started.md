# 🗣️ Echo: Getting Started example is broken

## 🤦 The Confusion:
I copied the first launch command from the README:
`cargo run -p nes-desktop --release -- ./roms/homebrew/homebrew.nes`
But it failed! The error message said:
"Error: Could not find the ROM file at './roms/homebrew/homebrew.nes'. Hint: Check the path or try the bundled homebrew ROM: ./roms/homebrew/homebrew.nes or <path-to-your-rom>.nes"
But the hint tells me to try the very file I'm trying to run, which is missing!

## 🕵️ The Reality:
Turns out the `homebrew.nes` file doesn't exist by default. I have to build it first!
At the very bottom of the README, there's a section called "Homebrew ROM" that says I need to run:
`cargo run -p nes-test-harness --bin build_homebrew_rom`
But that's way down at the bottom! How was I supposed to know that before running the first example?

## 💡 The Fix:
Add a step before the first `cargo run` example to build the homebrew ROM (`cargo run -p nes-test-harness --bin build_homebrew_rom`), and update the error hint to mention building the homebrew ROM instead of just telling me to run it.
