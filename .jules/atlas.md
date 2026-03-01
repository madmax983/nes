**[Remove Circular Dependency and Nested Sprawl]
**Tangle:** The `api` and `replay` modules in `nes-core` had a circular dependency. Additionally, `macro_engine` was nested unnecessarily inside an `experimental` module in `nes-mcp`, exposing internals to binaries.
**Blueprint:** Inlined `replay_commands` directly into `api.rs` to break the cycle and deleted `replay.rs`. Moved `macro_engine.rs` up to the top level of `nes-mcp` and removed the `experimental` module namespace, flattening the structure.
