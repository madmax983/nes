**[Remove Circular Dependency and Nested Sprawl]
**Tangle:** The `api` and `replay` modules in `nes-core` had a circular dependency. Additionally, `macro_engine` was nested unnecessarily inside an `experimental` module in `nes-mcp`, exposing internals to binaries.
**Blueprint:** Inlined `replay_commands` directly into `api.rs` to break the cycle and deleted `replay.rs`. Moved `macro_engine.rs` up to the top level of `nes-mcp` and removed the `experimental` module namespace, flattening the structure.

**[Fix Core Circular Dependencies & Leaky Abstraction]
**Tangle:** A circular dependency existed where `nes_core::api` depended on `apu` and `ppu`, but `apu` and `ppu` depended back on `api` for `FRAME_*` and `AUDIO_*` constants. Additionally, the `experimental` module was leaking as a public API (`pub mod experimental`).
**Blueprint:** Moved constants to their respective `ppu.rs` and `apu.rs` domain modules to break the circular dependency. Re-exported them safely via `lib.rs` for consumers. Encapsulated `experimental` to `pub(crate) mod` and added `#[allow(dead_code)]` to prevent leaking internal R&D tools into the public `nes-core` API surface.
