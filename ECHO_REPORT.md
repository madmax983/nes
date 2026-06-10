# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run `cargo run -p nes-desktop --release -- --config ./nes.toml` from the "Desktop/TUI launch commands" section. Got an error: `failed to read config './nes.toml': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The workspace doesn't have a `nes.toml` by default, it has a `nes.example.toml`. The README says "Runtime and ROM paths are configured through `nes.toml` at the workspace root", but never instructs the user to create or copy the file before running the command.

* 💡 **The Fix:** Add a step before the launch commands instructing users to copy the example config: `cp nes.example.toml nes.toml`.

---

# 🗣️ Echo: WebAssembly build fails out of the box

**Description:**

* 🤦 **The Confusion:** Tried to run the WebAssembly build command from the README: `cargo build -p nes-web --target wasm32-unknown-unknown`. Got an immediate error: `error[E0463]: can't find crate for std`.
* 🕵️ **The Reality:** The `wasm32-unknown-unknown` target isn't installed by default in Rust, so the command just explodes instead of working.
* 💡 **The Fix:** Add a step before the build command instructing users to run `rustup target add wasm32-unknown-unknown`.

---

# 🗣️ Echo: Trunk command not found

**Description:**

* 🤦 **The Confusion:** Tried to run the Web demo local serve command: `powershell -NoProfile -ExecutionPolicy Bypass -File ./scripts/run_web_demo.ps1 -OpenBrowser`. The script immediately failed with `trunk is required. Install via: cargo install trunk`.
* 🕵️ **The Reality:** The README tells me to run the web demo using trunk, but never actually tells me to install trunk first. I had no idea what "Trunk" was.
* 💡 **The Fix:** Add `cargo install trunk` to the README instructions before running the demo script.
