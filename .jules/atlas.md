**Extracted MCP Protocol Structs**
**Tangle:** Duplicated JSON-RPC request and error structs between `nes-mcp` and `nes-desktop`.
**Blueprint:** Extracted shared structs (`RpcRequest`, `RpcError`) into `nes-mcp/src/protocol.rs` and re-exported them.
