import re

with open('crates/nes-mcp/src/protocol.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'pub const JSONRPC_VERSION: &str = "2.0";',
    '/// Standard JSON-RPC 2.0 version specifier expected by MCP.\n/// \n/// ## Examples\n/// \n/// ```rust\n/// use nes_mcp::protocol::JSONRPC_VERSION;\n/// assert_eq!(JSONRPC_VERSION, "2.0");\n/// ```\npub const JSONRPC_VERSION: &str = "2.0";'
)

content = content.replace(
    'pub const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";',
    '/// Default MCP protocol version supported by this server.\n/// \n/// ## Examples\n/// \n/// ```rust\n/// use nes_mcp::protocol::DEFAULT_PROTOCOL_VERSION;\n/// assert_eq!(DEFAULT_PROTOCOL_VERSION, "2025-06-18");\n/// ```\npub const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";'
)

content = content.replace(
    'pub fn dispatch_output_value(output: DispatchOutput) -> Value {',
    '/// Converts a core `DispatchOutput` enum into its JSON representation.\n///\n/// Ensures the resulting object is tagged with a `kind` field corresponding to the variant name,\n/// making it easy for MCP clients to deserialize.\n///\n/// ## Examples\n///\n/// ```rust\n/// use nes_mcp::DispatchOutput;\n/// use nes_mcp::protocol::dispatch_output_value;\n/// use serde_json::json;\n///\n/// let value = dispatch_output_value(DispatchOutput::Ack);\n/// assert_eq!(value, json!({ "kind": "ack" }));\n/// ```\n#[must_use]\npub fn dispatch_output_value(output: DispatchOutput) -> Value {'
)

# Fix duplicate `#[must_use]` that might have been created above:
content = content.replace(
    '#[must_use]\n#[must_use]\npub fn dispatch_output_value',
    '#[must_use]\npub fn dispatch_output_value'
)

content = content.replace(
    'pub fn tool_input_schema(tool_name: &str) -> Value {',
    '/// Retrieves the JSON schema definition for a specific tool\'s input parameters.\n///\n/// This schema is provided to the MCP client (like an LLM) so it knows exactly what arguments\n/// to pass and what types they should be.\n///\n/// ## Examples\n///\n/// ```rust\n/// use nes_mcp::protocol::tool_input_schema;\n///\n/// let schema = tool_input_schema("read_memory");\n/// assert!(schema["properties"].get("address").is_some());\n/// ```\n#[must_use]\npub fn tool_input_schema(tool_name: &str) -> Value {'
)

content = content.replace(
    '#[must_use]\n#[must_use]\npub fn tool_input_schema',
    '#[must_use]\npub fn tool_input_schema'
)

content = content.replace(
    'pub fn jsonrpc_result(id: Value, result: Value) -> Value {',
    '/// Wraps a successful JSON-RPC payload in the standard envelope.\n///\n/// ## Examples\n///\n/// ```rust\n/// use nes_mcp::protocol::jsonrpc_result;\n/// use serde_json::json;\n///\n/// let response = jsonrpc_result(json!(1), json!({"success": true}));\n/// assert_eq!(response["jsonrpc"], "2.0");\n/// assert_eq!(response["id"], json!(1));\n/// ```\n#[must_use]\npub fn jsonrpc_result(id: Value, result: Value) -> Value {'
)

content = content.replace(
    '#[must_use]\n#[must_use]\npub fn jsonrpc_result',
    '#[must_use]\npub fn jsonrpc_result'
)


content = content.replace(
    'pub fn jsonrpc_error(id: Value, err: RpcError) -> Value {',
    '/// Wraps an `RpcError` in the standard JSON-RPC error envelope.\n///\n/// ## Examples\n///\n/// ```rust\n/// use nes_mcp::protocol::{jsonrpc_error, RpcError};\n/// use serde_json::json;\n///\n/// let err = RpcError::invalid_params("bad args");\n/// let response = jsonrpc_error(json!(2), err);\n/// assert_eq!(response["error"]["code"], -32602);\n/// ```\n#[must_use]\npub fn jsonrpc_error(id: Value, err: RpcError) -> Value {'
)

content = content.replace(
    '#[must_use]\n#[must_use]\npub fn jsonrpc_error',
    '#[must_use]\npub fn jsonrpc_error'
)


with open('crates/nes-mcp/src/protocol.rs', 'w') as f:
    f.write(content)
