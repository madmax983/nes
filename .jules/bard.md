## 2024-03-11 - JSON-RPC Error Serialization
**Confusion:** Directly serializing `RpcError` using `as_json()` or generic macro derivatives causes compilation errors due to optional data fields (like `data: Option<Value>`) not mapping cleanly without exhaustive pattern matches.
**Clarification:** `crates/nes-mcp/src/protocol.rs` provides a custom `to_json()` method on `RpcError` which explicitly inserts the optional data field into a mutable map. Always use `err.to_json()` when converting errors for protocol transport.
## 2024-03-11 - README Hardcoded ROM Path
**Confusion:** The README AI Control Training examples used a hardcoded Windows local path (`"./roms/Super Mario Bros.nes"`), which caused "No such file or directory" errors for users who copy-pasted the command without having the file.
**Clarification:** Replaced the hardcoded path with `"<path-to-your-smb1-rom>.nes"` and added an explanatory comment so users know to supply their own ROM.
## 2025-06-18 - [Add Missing Examples for Desktop/Web APIS]
**Confusion:** The core runtime structures `ProfileEnv` and `NetplayClient` lacked context on why they were needed (thread handling vs I/O boundaries)
**Clarification:** Added context directly referencing asynchronous network loops and `no_run` example blocks to explain how users connect safely.
