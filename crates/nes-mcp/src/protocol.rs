//! JSON-RPC 2.0 protocol types for the Model Context Protocol (MCP) server.
//!
//! This module defines the core request and error structures used to communicate
//! between the MCP client (like an LLM) and the NES emulator host. The MCP
//! server expects incoming requests to conform to `RpcRequest` and formats
//! responses or failures as `RpcError`.
//!
//! Because the payload data can vary dynamically depending on the tool
//! invoked, the structures rely on `serde_json::Value` for parameter and
//! data fields.

use serde::Deserialize;
use serde_json::{Value, json};

/// A JSON-RPC 2.0 request sent from the MCP client.
///
/// This structure represents a single tool invocation or command. The MCP
/// server deserializes incoming JSON payloads into this type before dispatching
/// the command to the appropriate emulator core function.
///
/// ## Examples
///
/// ```
/// use nes_mcp::protocol::RpcRequest;
/// use serde_json::json;
///
/// let json_payload = r#"{
///     "jsonrpc": "2.0",
///     "id": 1,
///     "method": "tools/call",
///     "params": {
///         "name": "pause"
///     }
/// }"#;
///
/// let request: RpcRequest = serde_json::from_str(json_payload).unwrap();
/// assert_eq!(request.method, "tools/call");
/// assert_eq!(request.id, Some(json!(1)));
/// ```
#[derive(Debug, Deserialize)]
pub struct RpcRequest {
    /// The protocol version, typically exactly `"2.0"`.
    pub jsonrpc: String,
    /// The unique request identifier.
    ///
    /// This is optional in JSON-RPC (where lack of an ID implies a notification),
    /// but standard MCP tool calls will always provide one.
    #[serde(default)]
    pub id: Option<Value>,
    /// The name of the method to be invoked (e.g., `"tools/call"` or `"tools/list"`).
    pub method: String,
    /// The parameters to be passed to the method.
    ///
    /// The structure of these parameters depends entirely on the `method` being called.
    #[serde(default)]
    pub params: Option<Value>,
}

/// A JSON-RPC 2.0 error response.
///
/// This structure is used to communicate failures back to the MCP client.
/// It encapsulates the standard JSON-RPC error codes along with a human-readable
/// message and an optional data payload for extended diagnostic context.
///
/// When converting this error into a JSON value for transmission, you must use
/// the [`RpcError::to_json`] method to avoid non-exhaustive pattern compilation errors
/// that can arise from custom serialization strategies.
#[derive(Debug, Clone)]
pub struct RpcError {
    /// A Number that indicates the error type that occurred.
    ///
    /// Common codes are -32700 (Parse error) or -32600 (Invalid Request).
    pub code: i64,
    /// A String providing a short description of the error.
    pub message: String,
    /// A Primitive or Structured value that contains additional information
    /// about the error.
    pub data: Option<Value>,
}

impl RpcError {
    /// Creates a new "Parse error" (-32700) response.
    ///
    /// Use this when the incoming payload cannot be parsed as valid JSON.
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: message.into(),
            data: None,
        }
    }

    /// Creates a new "Invalid Request" (-32600) response.
    ///
    /// Use this when the JSON payload is valid, but the structure does not
    /// conform to the JSON-RPC 2.0 specification.
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
            data: None,
        }
    }

    /// Creates a new "Method not found" (-32601) response.
    ///
    /// Use this when the client attempts to invoke a tool or method that
    /// the server does not expose in its catalog.
    pub fn method_not_found(message: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: message.into(),
            data: None,
        }
    }

    /// Creates a new "Invalid params" (-32602) response.
    ///
    /// Use this when the tool exists, but the provided arguments fail validation
    /// against the tool's expected schema.
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }

    /// Serializes the error into a valid JSON-RPC 2.0 error object.
    ///
    /// **Important:** You must use this custom `to_json()` method instead of `as_json()`
    /// when constructing the response payload. It gracefully handles the optional `data`
    /// field without generating compilation errors associated with non-exhaustive patterns.
    ///
    /// ## Examples
    ///
    /// ```
    /// use nes_mcp::protocol::RpcError;
    ///
    /// let err = RpcError::invalid_params("missing required field 'rom_path'");
    /// let json_val = err.to_json();
    ///
    /// assert_eq!(json_val["code"], -32602);
    /// assert_eq!(json_val["message"], "missing required field 'rom_path'");
    /// assert!(json_val.get("data").is_none());
    /// ```
    pub fn to_json(&self) -> Value {
        let mut value = json!({
            "code": self.code,
            "message": self.message,
        });
        if let Some(data) = self.data.clone()
            && let Some(obj) = value.as_object_mut()
        {
            obj.insert("data".to_owned(), data);
        }
        value
    }

    /// Creates a new "Internal error" (-32603) response.
    ///
    /// Use this when the emulator core encounters an unexpected panic or
    /// fatal state during the execution of a valid tool command.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
            data: None,
        }
    }
}
