# nes-web

`nes-web` exposes a wasm-bindgen wrapper around `nes-core` for browser hosts.

## WASM Path

This is the end-to-end execution path used by the web demo:

1. `web/index.html` is the Trunk entrypoint and declares the Rust build input with:
   - `<link data-trunk rel="rust" href="../crates/nes-web/Cargo.toml" data-target-name="nes_web">`
2. `crates/nes-web/Trunk.toml` points Trunk at `../../web/index.html` and writes built assets to `../../web/dist`.
3. `web/app.js` imports the generated wasm JS glue (`./nes-web.js`) and constructs `NesWebEmulator`.
4. `crates/nes-web/src/lib.rs` exports `NesWebEmulator` via `wasm-bindgen`.
5. `NesWebEmulator` forwards calls into `WebRuntime` (`crates/nes-web/src/runtime.rs`).
6. Input translation path is:
   - DOM `keydown`/`keyup` in `web/app.js`
   - `dispatch_dom_key(...)` on `NesWebEmulator`
   - `map_dom_key_to_command(...)` in `crates/nes-web/src/bridge.rs`
   - `nes_core::Command::{PressButton,ReleaseButton}` execution in `NesCore`
7. Output path is:
   - video: `frame_rgba()` from `WebRuntime` -> canvas `putImageData(...)` in `web/app.js`
   - audio: `audio_chunk_i16()` from `WebRuntime` -> WebAudio worklet queue in `web/app.js`

## Build for WebAssembly

```powershell
cargo build -p nes-web --target wasm32-unknown-unknown
```

Bundle with Trunk:

```powershell
Push-Location .\crates\nes-web
trunk build --config .\Trunk.toml
Pop-Location
```

The exported class is `NesWebEmulator`.

For a local browser demo, use:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\run_web_demo.ps1 -OpenBrowser
```

Or run via Trunk directly:

```bash
cargo install trunk
cd crates/nes-web
trunk serve --open
```

The script uses `trunk serve` for the browser dev server.

## Local ROM Persistence

The web host now supports local ROM persistence through IndexedDB:

- Selecting a ROM file stores it locally on the current device.
- Loading the homebrew ROM stores it locally as well.
- On next launch, the last saved ROM is restored automatically.
- Use the `Forget Saved ROM` button to delete local ROM bytes.

This keeps ROM distribution out of the deployed web assets while still enabling repeat play on mobile.
