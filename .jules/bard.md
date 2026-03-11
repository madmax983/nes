## 2024-03-11 - JSON-RPC Error Serialization
**Confusion:** Directly serializing `RpcError` using `as_json()` or generic macro derivatives causes compilation errors due to optional data fields (like `data: Option<Value>`) not mapping cleanly without exhaustive pattern matches.
**Clarification:** `crates/nes-mcp/src/protocol.rs` provides a custom `to_json()` method on `RpcError` which explicitly inserts the optional data field into a mutable map. Always use `err.to_json()` when converting errors for protocol transport.
