# 🗣️ Echo: Getting Started example is broken

**Description:**

* 🤦 **The Confusion:** Tried to run `cargo run -p nes-desktop --release -- --config ./nes.toml` from the "Desktop/TUI launch commands" section. Got an error: `failed to read config './nes.toml': No such file or directory (os error 2)`.

* 🕵️ **The Reality:** The workspace doesn't have a `nes.toml` by default, it has a `nes.example.toml`. The README says "Runtime and ROM paths are configured through `nes.toml` at the workspace root", but never instructs the user to create or copy the file before running the command.

* 💡 **The Fix:** Add a step before the launch commands instructing users to copy the example config: `cp nes.example.toml nes.toml`.

# 🗣️ Echo: AI Training and WASM examples are broken

**Description:**

* 🤦 **The Confusion:**
  1. Tried to run the AI Control Training step 2 (`train_smb_control`). Got an error: `Failed to read profile config: No such file or directory (os error 2)`.
  2. I figured out I needed to copy `smb-control.example.toml` to `smb-control.toml`. Ran step 2 again. Got another error: `Training failed: failed to read ROM './roms/Super Mario Bros.nes': No such file or directory`. Wait, step 1 said we are using the "bundled homebrew ROM" for the demo! Why is it looking for Mario?
  3. Tried to run the WebAssembly build (`cargo build -p nes-web --target wasm32-unknown-unknown`). It failed with `error[E0463]: can't find crate for std` because the target wasn't installed.

* 🕵️ **The Reality:**
  1. The README never instructs the user to copy the `smb-control.example.toml` file.
  2. `smb-control.example.toml` hardcodes the ROM path to `./roms/Super Mario Bros.nes`, contradicting the README's claim that we are using the homebrew ROM for demonstration.
  3. The WASM build assumes the user already knows they need to run `rustup target add wasm32-unknown-unknown`.

* 💡 **The Fix:**
  1. Add a `cp` command for the AI profile before step 2 in the README.
  2. Update `smb-control.example.toml` to point to `./roms/homebrew/homebrew.nes` so the demo works out of the box.
  3. Add `rustup target add wasm32-unknown-unknown` before the `cargo build` command in the WASM section.
