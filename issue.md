# 🗣️ Echo: Getting Started web example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run the WebAssembly build from the `README.md` (`cargo build -p nes-web --target wasm32-unknown-unknown`). It immediately failed with `error[E0463]: can't find crate for std`.
* 🕵️ **The Reality:** Turns out I needed to install the wasm32 target first via `rustup target add wasm32-unknown-unknown`.
* 💡 **The Fix:** Add `rustup target add wasm32-unknown-unknown` to the README instructions before the build command.
