# Homebrew ROM

This directory holds a tiny custom NES ROM generated from in-repo source.

## Build

```powershell
cargo run -p nes-test-harness --bin build_homebrew_rom
```

Default output path:

`roms/homebrew/homebrew.nes`

## Run

```powershell
cargo run -p nes-desktop --release -- .\roms\homebrew\homebrew.nes
```

## Controls

- Arrows: move sprite
- `Esc`: quit desktop
