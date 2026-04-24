# 🗣️ Echo: Getting Started example is broken

## Description

* 🤦 **The Confusion:** Tried to run the `nes-desktop` with the `mcp-host` feature from the `README.md` instructions:
  `cargo run -p nes-desktop --features mcp-host -- ./roms/homebrew/homebrew.nes --mcp-host --mcp-bind 127.0.0.1:6502`.
  The compiler failed with "unresolved import `crate::metrics`" and "use of undeclared type `NetplayClient` / `NetplayRuntimeStats`".
* 🕵️ **The Reality:** The `crates/nes-desktop/src/main.rs` file gated imports behind `#[cfg(feature = "mcp-host")]`, but the items were actually used unconditionally lower in the code (which fails type inference without the imports and triggers the unresolved import for `metrics`).
* 💡 **The Fix:** Removed the `#[cfg(feature = "mcp-host")]` attributes from the `metrics` module declaration and the `NetplayClient`/`NetplayRuntimeStats` imports in `crates/nes-desktop/src/main.rs`. Also ran the `cargo run -p nes-mcp --bin nes-mcp` to ensure the tool executes successfully.