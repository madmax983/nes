# nes-web

`nes-web` exposes a wasm-bindgen wrapper around `nes-core` for browser hosts.

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

The script uses `trunk serve` for the browser dev server.
