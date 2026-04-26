use std::io::{self, BufRead, BufReader, Write};

use comfy_table::{Cell, Color as TableColor, Table, presets::UTF8_FULL};
use crossterm::style::{Color, Stylize};
use nes_core::NesCore;
use nes_mcp::{
    dispatch_tool,
    protocol::{
        DEFAULT_PROTOCOL_VERSION, JSONRPC_VERSION, RpcError, RpcRequest, dispatch_output_value,
        jsonrpc_error, jsonrpc_result, map_tool_arguments, tool_input_schema,
    },
    tool_catalog,
};
use serde_json::{Value, json};

/// Fatal errors encountered by the MCP daemon process.
///
/// These errors represent conditions where the JSON-RPC server can no longer
/// continue operating, such as when `stdin`/`stdout` pipes are closed or when
/// incoming payloads severely violate the protocol boundaries (e.g. content
/// length headers are malformed).
#[derive(Debug)]
pub enum McpError {
    /// An underlying I/O error reading from or writing to stdio.
    Io(std::io::Error),
    /// A violation of the MCP framing or JSON-RPC specification.
    Protocol(String),
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IO error: {err}"),
            Self::Protocol(err) => write!(f, "Protocol error: {err}"),
        }
    }
}

impl std::error::Error for McpError {}

impl From<std::io::Error> for McpError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<String> for McpError {
    fn from(err: String) -> Self {
        Self::Protocol(err)
    }
}

impl From<&str> for McpError {
    fn from(err: &str) -> Self {
        Self::Protocol(err.to_owned())
    }
}

#[derive(Default)]
struct ServerState {
    core: NesCore,
    protocol_version: String,
    initialized: bool,
}

impl ServerState {
    fn new() -> Self {
        Self {
            core: NesCore::new(),
            protocol_version: DEFAULT_PROTOCOL_VERSION.to_owned(),
            initialized: false,
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("\n{}", format!("Error: {err}").with(Color::Red).bold());
        std::process::exit(1);
    }
}

fn run() -> Result<(), McpError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut state = ServerState::new();

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![
        Cell::new("Property").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("Protocol Version"),
        Cell::new(DEFAULT_PROTOCOL_VERSION).fg(TableColor::Yellow),
    ]);
    table.add_row(vec![
        Cell::new("Tools Loaded"),
        Cell::new(tool_catalog().len().to_string()).fg(TableColor::Yellow),
    ]);
    table.add_row(vec![
        Cell::new("Status"),
        Cell::new("Listening on stdio").fg(TableColor::Green),
    ]);

    eprintln!("{}", "nes-mcpd".with(Color::Cyan).bold());
    eprintln!("\n{table}");

    while let Some(payload) = read_stdio_message(&mut reader)? {
        let Some(response) = handle_message(&mut state, &payload) else {
            continue;
        };
        write_stdio_message(&mut writer, &response)?;
    }
    Ok(())
}

fn handle_message(state: &mut ServerState, payload: &[u8]) -> Option<Value> {
    let request: RpcRequest = match serde_json::from_slice(payload) {
        Ok(req) => req,
        Err(err) => {
            return Some(jsonrpc_error(
                Value::Null,
                RpcError::parse_error(format!("invalid JSON payload: {err}")),
            ));
        }
    };

    // **Performance optimization:** `RpcRequest` is an owned value obtained from deserialization,
    // so we can consume `id` directly instead of allocating a new `Value` via `.clone()`.
    let id = request.id;
    let is_notification = id.is_none();
    let response_id = id.unwrap_or_default();

    if request.jsonrpc != JSONRPC_VERSION {
        if is_notification {
            return None;
        }
        return Some(jsonrpc_error(
            response_id,
            RpcError::invalid_request("jsonrpc must be '2.0'"),
        ));
    }

    let method_result = match request.method.as_str() {
        "initialize" => handle_initialize(state, request.params.as_ref()),
        "notifications/initialized" => {
            state.initialized = true;
            Ok(None)
        }
        "ping" => Ok(Some(json!({}))),
        "tools/list" => Ok(Some(handle_tools_list())),
        "tools/call" => handle_tools_call(state, request.params),
        "resources/list" => Ok(Some(json!({ "resources": [] }))),
        "prompts/list" => Ok(Some(json!({ "prompts": [] }))),
        "logging/setLevel" => Ok(Some(json!({}))),
        _ => Err(RpcError::method_not_found(format!(
            "method '{}' is not implemented",
            request.method
        ))),
    };

    if is_notification {
        return None;
    }

    match method_result {
        Ok(Some(result)) => Some(jsonrpc_result(response_id, result)),
        Ok(None) => Some(jsonrpc_result(response_id, json!({}))),
        Err(err) => Some(jsonrpc_error(response_id, err)),
    }
}

fn handle_initialize(
    state: &mut ServerState,
    params: Option<&Value>,
) -> Result<Option<Value>, RpcError> {
    let params_obj = params
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError::invalid_params("initialize params must be an object"))?;

    let requested_protocol = params_obj
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION)
        .to_owned();
    state.protocol_version = requested_protocol.clone();

    Ok(Some(json!({
        "protocolVersion": requested_protocol,
        "capabilities": {
            "tools": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": "nes-mcpd",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "Use tools to load ROM bytes/path and drive the emulator core deterministically."
    })))
}

fn handle_tools_list() -> Value {
    let tools: Vec<Value> = tool_catalog()
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "inputSchema": tool_input_schema(tool.name),
            })
        })
        .collect();
    json!({ "tools": tools })
}

fn handle_tools_call(
    state: &mut ServerState,
    params: Option<Value>,
) -> Result<Option<Value>, RpcError> {
    let mut params_obj = match params {
        Some(Value::Object(map)) => map,
        _ => {
            return Err(RpcError::invalid_params(
                "tools/call params must be an object",
            ));
        }
    };
    let tool_name = match params_obj.remove("name") {
        Some(Value::String(name)) => name,
        _ => {
            return Err(RpcError::invalid_params(
                "tools/call requires string field 'name'",
            ));
        }
    };

    let args = match params_obj.remove("arguments") {
        Some(Value::Object(map)) => map,
        _ => Default::default(),
    };
    let tool_params = map_tool_arguments(args);

    let call_result = match dispatch_tool(&mut state.core, &tool_name, &tool_params) {
        Ok(output) => {
            let structured = dispatch_output_value(output);
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("ok: {tool_name}")
                    }
                ],
                "structuredContent": structured,
                "isError": false
            })
        }
        Err(err) => {
            json!({
                "content": [
                    {
                        "type": "text",
                        "text": format!("{err}")
                    }
                ],
                "isError": true
            })
        }
    };
    Ok(Some(call_result))
}

fn read_stdio_message(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, McpError> {
    let mut content_length = None::<usize>;
    let mut line = String::new();

    loop {
        line.clear();

        let read = reader
            .read_line(&mut line)
            .map_err(|err| McpError::Protocol(format!("failed reading header line: {err}")))?;
        if read == 0 {
            if content_length.is_none() {
                return Ok(None);
            }
            return Err(McpError::Protocol(
                "unexpected EOF while reading MCP headers".to_owned(),
            ));
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            let parsed = value.trim();
            let len = parsed.parse::<usize>().map_err(|_| {
                McpError::Protocol(format!("invalid Content-Length value '{parsed}'"))
            })?;
            content_length = Some(len);
        }
    }
    let len = content_length
        .ok_or_else(|| McpError::Protocol("missing Content-Length header".to_owned()))?;
    const MAX_PAYLOAD_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    if len > MAX_PAYLOAD_SIZE {
        return Err(McpError::Protocol(format!(
            "Content-Length {len} exceeds maximum allowed size of {MAX_PAYLOAD_SIZE} bytes"
        )));
    }
    let mut payload = vec![0_u8; len];
    reader
        .read_exact(&mut payload)
        .map_err(|err| McpError::Protocol(format!("failed reading payload body: {err}")))?;
    Ok(Some(payload))
}

/// Writes a framed JSON-RPC message to stdio.
///
/// **Performance optimization:** Uses `write!` macro directly to the writer instead
/// of allocating an intermediate `String` via `format!` to construct the `Content-Length` header.
fn write_stdio_message(writer: &mut impl Write, value: &Value) -> Result<(), McpError> {
    let payload = serde_json::to_vec(value)
        .map_err(|err| McpError::Protocol(format!("failed serializing JSON response: {err}")))?;
    write!(writer, "Content-Length: {}\r\n\r\n", payload.len())
        .and_then(|_| writer.write_all(&payload))
        .and_then(|_| writer.flush())
        .map_err(|err| McpError::Protocol(format!("failed writing stdio response: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(method: &str, id: Value, params: Value) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        })
    }

    fn call(state: &mut ServerState, request: Value) -> Value {
        let payload = serde_json::to_vec(&request).expect("request serializes");
        handle_message(state, &payload).expect("request produces response")
    }

    #[test]
    fn initialize_returns_server_capabilities() {
        let mut state = ServerState::new();
        let response = call(
            &mut state,
            request(
                "initialize",
                json!(1),
                json!({
                    "protocolVersion": DEFAULT_PROTOCOL_VERSION,
                    "capabilities": {}
                }),
            ),
        );

        assert_eq!(response["jsonrpc"], json!("2.0"));
        assert_eq!(response["id"], json!(1));
        assert_eq!(response["result"]["serverInfo"]["name"], json!("nes-mcpd"));
        assert_eq!(
            response["result"]["capabilities"]["tools"]["listChanged"],
            json!(false)
        );
        assert_eq!(state.protocol_version, DEFAULT_PROTOCOL_VERSION);
    }

    #[test]
    fn tools_list_includes_load_rom_with_schema() {
        let mut state = ServerState::new();
        let response = call(&mut state, request("tools/list", json!(2), json!({})));
        let tools = response["result"]["tools"]
            .as_array()
            .expect("tools array exists");
        let load_rom = tools
            .iter()
            .find(|tool| tool["name"] == json!("load_rom"))
            .expect("load_rom tool exists");
        assert_eq!(
            load_rom["description"],
            json!("Load an iNES ROM into the emulator core")
        );
        assert_eq!(load_rom["inputSchema"]["type"], json!("object"));
        assert!(load_rom["inputSchema"]["oneOf"].is_array());

        let export_dsl = tools
            .iter()
            .find(|tool| tool["name"] == json!("export_6502_dsl_rom"))
            .expect("export_6502_dsl_rom tool exists");
        assert_eq!(export_dsl["inputSchema"]["type"], json!("object"));
        assert_eq!(
            export_dsl["inputSchema"]["required"],
            json!(["source", "output_path"])
        );

        let export_dsl_base64 = tools
            .iter()
            .find(|tool| tool["name"] == json!("export_6502_dsl_rom_base64"))
            .expect("export_6502_dsl_rom_base64 tool exists");
        assert_eq!(export_dsl_base64["inputSchema"]["type"], json!("object"));
        assert_eq!(
            export_dsl_base64["inputSchema"]["required"],
            json!(["source"])
        );
    }

    #[test]
    fn tools_call_reports_dispatch_errors_as_tool_errors() {
        let mut state = ServerState::new();
        let response = call(
            &mut state,
            request(
                "tools/call",
                json!(3),
                json!({
                    "name": "load_rom",
                    "arguments": {}
                }),
            ),
        );

        assert_eq!(response["result"]["isError"], json!(true));
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .expect("error text present");
        assert!(text.contains("provide rom_hex or rom_path"));
    }

    #[test]
    fn initialized_notification_does_not_emit_response() {
        let mut state = ServerState::new();
        let payload = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
        .expect("notification serializes");
        let response = handle_message(&mut state, &payload);
        assert!(response.is_none());
        assert!(state.initialized);
    }

    #[test]
    fn rpc_error_helpers_use_jsonrpc_standard_codes() {
        let parse = RpcError::parse_error("bad json");
        let invalid = RpcError::invalid_request("bad request");
        let missing = RpcError::method_not_found("missing");
        let params = RpcError::invalid_params("bad params");

        assert_eq!(parse.code, -32_700);
        assert_eq!(invalid.code, -32_600);
        assert_eq!(missing.code, -32_601);
        assert_eq!(params.code, -32_602);

        assert_eq!(parse.into_json()["message"], json!("bad json"));
        assert_eq!(invalid.into_json()["message"], json!("bad request"));
        assert_eq!(missing.into_json()["message"], json!("missing"));
        assert_eq!(params.into_json()["message"], json!("bad params"));
    }

    #[test]
    fn handle_message_supports_ping_and_auxiliary_list_methods() {
        let mut state = ServerState::new();

        let ping = call(&mut state, request("ping", json!(10), json!({})));
        assert_eq!(ping["result"], json!({}));

        let resources = call(&mut state, request("resources/list", json!(11), json!({})));
        assert_eq!(resources["result"], json!({ "resources": [] }));

        let prompts = call(&mut state, request("prompts/list", json!(12), json!({})));
        assert_eq!(prompts["result"], json!({ "prompts": [] }));

        let logging = call(
            &mut state,
            request("logging/setLevel", json!(13), json!({})),
        );
        assert_eq!(logging["result"], json!({}));
    }

    #[test]
    fn mcp_error_formatting_and_conversions() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let mcp_io: McpError = io_err.into();
        assert_eq!(mcp_io.to_string(), "IO error: file missing");

        let mcp_proto: McpError = "bad format".into();
        assert_eq!(mcp_proto.to_string(), "Protocol error: bad format");

        let mcp_proto_string: McpError = "bad format".to_owned().into();
        assert_eq!(mcp_proto_string.to_string(), "Protocol error: bad format");
    }

    #[test]
    fn read_stdio_message_handles_errors() {
        let mut reader = b"Content-Length: abc\r\n\r\n".as_slice();
        let err = read_stdio_message(&mut reader).unwrap_err();
        assert!(
            err.to_string()
                .contains("invalid Content-Length value 'abc'")
        );

        let mut reader = b"Something else\r\n\r\n".as_slice();
        let err = read_stdio_message(&mut reader).unwrap_err();
        assert!(err.to_string().contains("missing Content-Length header"));

        let mut reader = b"Content-Length: 100\r\nEOF".as_slice();
        let err = read_stdio_message(&mut reader).unwrap_err();
        assert!(
            err.to_string()
                .contains("unexpected EOF while reading MCP headers")
        );

        let mut reader = b"Content-Length: 100\r\n\r\nshort".as_slice();
        let err = read_stdio_message(&mut reader).unwrap_err();
        assert!(err.to_string().contains("failed reading payload body"));
    }
}
