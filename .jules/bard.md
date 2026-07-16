## 2024-03-11 - JSON-RPC Error Serialization
**Confusion:** Directly serializing `RpcError` using `as_json()` or generic macro derivatives causes compilation errors due to optional data fields (like `data: Option<Value>`) not mapping cleanly without exhaustive pattern matches.
**Clarification:** `crates/nes-mcp/src/protocol.rs` provides a custom `to_json()` method on `RpcError` which explicitly inserts the optional data field into a mutable map. Always use `err.to_json()` when converting errors for protocol transport.
## 2024-03-11 - README Hardcoded ROM Path
**Confusion:** The README AI Control Training examples used a hardcoded Windows local path (`"./roms/Super Mario Bros.nes"`), which caused "No such file or directory" errors for users who copy-pasted the command without having the file.
**Clarification:** Replaced the hardcoded path with `"<path-to-your-smb1-rom>.nes"` and added an explanatory comment so users know to supply their own ROM.
## 2025-06-18 - [Add Missing Examples for Desktop/Web APIS]
**Confusion:** The core runtime structures `ProfileEnv` and `NetplayClient` lacked context on why they were needed (thread handling vs I/O boundaries)
**Clarification:** Added context directly referencing asynchronous network loops and `no_run` example blocks to explain how users connect safely.
## 2025-06-18 - [Add Missing Examples for Desktop/Web APIS]
**Confusion:** The core runtime structures `ProfileEnv` and `NetplayClient` lacked context on why they were needed (thread handling vs I/O boundaries)
**Clarification:** Added context directly referencing asynchronous network loops and `no_run` example blocks to explain how users connect safely.
## 2025-06-18 - JSON Protocol Serialization and Relay Configs
**Confusion:** `ClientMessage` and `ServerMessage` were undocumented, leaving developers unaware of how matchmaking and determinism worked. The Relay server config also lacked context on its simulator behavior.
**Clarification:** Added narrative module and item-level docs explaining the 'why' (e.g., deterministic pulse of inputs, artificial latency injection) along with executable doctests that assert the exact JSON wire format.
## 2025-06-18 - Missing Crate-Level Documentation
**Confusion:** Rustdoc throws warnings for missing `//!` crate-level documentation even on library roots or binaries when compiled with `-W rustdoc::missing_crate_level_docs`. Users were unable to understand the high level purpose of a crate from generated docs.
**Clarification:** Added module-level `//!` docs at the top of all root library files and binary entry points to explain the high-level intent.
## 2025-06-18 - Adding Doc Tests to Missing APIs
**Confusion:** Some newer experimental or foundational APIs (like `EventTracker` and `nes-proof` scaffolding) lacked documentation entirely, resulting in "missing documentation" warnings when compiling docs with `#![warn(missing_docs)]` and leaving users to figure out usage entirely by source code exploration.
**Clarification:** Added thorough `///` comments containing `## Examples` sections to all public methods in `EventTracker` and to the `proof_crate_marker` function, providing executable context.
## 2025-06-18 - Missing serde Module Docs
**Confusion:** Internal helper modules for `serde` sequence iteration (like `serde_iter` in `nes-desktop`) often lack documentation because they are seen as implementation details. However, without docs, users don't understand how `serialize` treats iterators when compiling with strict warnings or viewing internal docs.
**Clarification:** Added explicit module-level and function-level `///` docs to `serde_iter::serialize` explaining the `Serializer::collect_seq` bridging behavior, along with an executable `## Examples` doctest that asserts the JSON output. Made the module `pub` so doctests can access it properly.

## 2024-04-27 - Documented Missing Core and Desktop Functions
**Confusion:** Functions `add_rule` and `evaluate` in `nes-core/src/experimental/spatial_bot.rs`, and `read_framed_message` in `nes-desktop/src/mcp_host.rs` were missing documentation, which made it unclear what they were doing without looking at their implementations. Furthermore, the `read_framed_message` doctest failed initially because the `Content-Length` provided in the doctest did not exactly match the length of the string bytes `{"key":"val"}` (length is 13, not 12).
**Clarification:** Added clear doc comments (`///`) describing what the functions do and added executable doctests for each to demonstrate valid usage. Updated the `Content-Length` in the doctest for `read_framed_message` from 12 to 13 to correctly match the payload size and allow the test to pass.

## 2024-07-16 - FME7 Intra-doc link panic
**Confusion:** Developers and cargo doc experienced a warning regarding `[DOTS_PER_CPU_CYCLE]` in `mapper/fme7.rs`, linking to a private item `DOTS_PER_CPU_CYCLE` while the struct it resided on was documented as part of the public API. Additionally `nes-core` failed to pass `-W missing_docs` on missing docs in spatial_bot and ppu_visualizer.
**Clarification:** Replaced `[DOTS_PER_CPU_CYCLE]` with `3` directly in the documentation comment for `advance_hardware_cycles`, added public structure and field level documentation to `BotRule`, `SpatialBot`, and `PpuVisualizer` under the experimental module.
