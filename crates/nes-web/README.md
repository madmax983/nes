# nes-web

`nes-web` exposes a wasm-bindgen wrapper around `nes-core` for browser hosts.

## Build for WebAssembly

```powershell
cargo build -p nes-web --target wasm32-unknown-unknown
```

With wasm-pack (recommended):

```powershell
wasm-pack build .\crates\nes-web --target web --out-dir ..\..\web\pkg
```

With wasm-bindgen CLI:

```powershell
wasm-bindgen --target web --out-dir .\web-dist .\target\wasm32-unknown-unknown\debug\nes_web.wasm
```

The exported class is `NesWebEmulator`.

For a local browser demo, use:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\run_web_demo.ps1 -OpenBrowser
```

The script prefers Python (`py`/`python`) for static hosting and falls back to Node.
