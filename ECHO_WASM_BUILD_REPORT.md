# 🗣️ Echo: WebAssembly build instructions are broken

🤦 **The Confusion:** Tried to run `cargo build -p nes-web --target wasm32-unknown-unknown` from the WebAssembly build section. Got a compiler error: `can't find crate for std`.

🕵️ **The Reality:** Turns out I needed to install the wasm32 target first using `rustup target add wasm32-unknown-unknown`.

💡 **The Fix:** Add a step to the README.md before the WASM build command instructing users to add the target: `rustup target add wasm32-unknown-unknown`.
