**[McpHost Cohesion]**
**Tangle:** The `McpHost` logic (the transport and protocol handling for the Model Context Protocol server) was housed inside the UI-heavy `nes-desktop` runner `src/mcp_host.rs`, which meant `nes-desktop` imported `nes-mcp` tools but then handled the protocol logic itself.
**Blueprint:** Moved `mcp_host.rs` directly into `nes-mcp/src/mcp_host.rs`, centralizing all MCP transport and dispatch logic under the `nes-mcp` crate. This enforces high cohesion within the MCP crate and slims down the desktop app to simply launching the pre-built host.
