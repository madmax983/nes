**Extracted MCP Protocol Structs**
**Tangle:** Duplicated JSON-RPC request and error structs between `nes-mcp` and `nes-desktop`.
**Blueprint:** Extracted shared structs (`RpcRequest`, `RpcError`) into `nes-mcp/src/protocol.rs` and re-exported them.

**Extracted PPM Encoding & Unified Button Parsing**
**Tangle:** Duplicated `encode_ppm` logic across `nes-desktop` and `nes-mcp`. Inconsistent and duplicated button parsing rules across multiple modules without a standardized interface.
**Blueprint:** Extracted shared PPM encoding into `nes_core::ppm`. Unified button parsing by implementing `std::str::FromStr` on `nes_core::api::Button` and refactored consumers to use standard `parse()` patterns.
