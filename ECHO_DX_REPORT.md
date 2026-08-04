# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** "Tried to run `cargo run -p nes-desktop --release -- ./roms/homebrew/homebrew.nes`. It crashed with `Failed to initialize any backend! Wayland status: XdgRuntimeDirNotSet X11 status: XOpenDisplayFailed`."
* 🕵️ **The Reality:** "Turns out running the desktop GUI app in a headless environment without X11/Wayland configured causes a hard crash from `winit`."
* 💡 **The Fix:** "Add a clear note in the README mentioning that the emulator requires an active X11 or Wayland session to run, or add a headless fallback mode/instructions for environments without a display server."
