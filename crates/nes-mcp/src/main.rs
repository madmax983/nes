use std::io::{self, BufRead, BufReader, Write};

use comfy_table::{Cell, Color as TableColor, Table};
use crossterm::style::{Color, Stylize};
use nes_core::NesCore;
use nes_mcp::{DispatchOutput, ToolParams, dispatch_tool, tool_catalog};
use serde::Deserialize;
use serde_json::{Map, Value, json};

const JSONRPC_VERSION: &str = "2.0";
const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug)]
pub enum McpError {
    Io(std::io::Error),
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

#[derive(Debug, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Option<Value>,
}

#[derive(Debug, Clone)]
struct RpcError {
    code: i64,
    message: String,
    data: Option<Value>,
}

impl RpcError {
    fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: message.into(),
            data: None,
        }
    }

    fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
            data: None,
        }
    }

    fn method_not_found(message: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: message.into(),
            data: None,
        }
    }

    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }

    fn to_json(&self) -> Value {
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
        let _ = writeln!(io::stderr(), "nes-mcpd fatal error: {err}");
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
    table.set_header(vec![
        Cell::new("Setting").fg(TableColor::Cyan),
        Cell::new("Value").fg(TableColor::White),
    ]);

    table.add_row(vec![
        Cell::new("Protocol Version"),
        Cell::new(DEFAULT_PROTOCOL_VERSION).fg(TableColor::Green),
    ]);
    table.add_row(vec![
        Cell::new("Tools Loaded"),
        Cell::new(tool_catalog().len().to_string()).fg(TableColor::Yellow),
    ]);

    eprintln!("{}", "nes-mcpd".with(Color::Cyan).bold());
    eprintln!("{table}\n");

    loop {
        let Some(payload) = read_stdio_message(&mut reader)? else {
            break;
        };
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
            return Some(jsonrpc_error_response(
                Value::Null,
                RpcError::parse_error(format!("invalid JSON payload: {err}")),
            ));
        }
    };

    let id = request.id.clone();
    let is_notification = id.is_none();
    let response_id = id.unwrap_or(Value::Null);

    if request.jsonrpc != JSONRPC_VERSION {
        if is_notification {
            return None;
        }
        return Some(jsonrpc_error_response(
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
        Ok(Some(result)) => Some(jsonrpc_result_response(response_id, result)),
        Ok(None) => Some(jsonrpc_result_response(response_id, json!({}))),
        Err(err) => Some(jsonrpc_error_response(response_id, err)),
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
        _ => return Err(RpcError::invalid_params("tools/call params must be an object")),
    };
    let tool_name = match params_obj.remove("name") {
        Some(Value::String(name)) => name,
        _ => return Err(RpcError::invalid_params("tools/call requires string field 'name'")),
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

/// Maps raw JSON RPC arguments into a strongly typed `ToolParams` map.
///
/// **Performance optimization:** This function consumes an owned `Map<String, Value>`
/// rather than taking a borrowed reference `&Map`. Because the MCP dispatch
/// layer typically already has an owned arguments object, taking ownership here
/// entirely eliminates the need to allocate and `.clone()` every string key
/// and value when inserting them into the returned `ToolParams`.
fn map_tool_arguments(arguments: Map<String, Value>) -> ToolParams {
    let mut params = ToolParams::new();
    for (key, value) in arguments {
        params.insert(key, json_arg_to_string(value));
    }
    params
}

/// Converts a JSON `Value` into a `String` without extra allocations if possible.
///
/// **Performance optimization:** Taking an owned `Value` allows us to extract
/// and return the inner `String` directly via pattern matching, avoiding
/// a heap allocation compared to calling `.to_string()` or `.clone()` on a reference.
fn json_arg_to_string(value: Value) -> String {
    match value {
        Value::String(v) => v,
        _ => value.to_string(),
    }
}

fn dispatch_output_value(output: DispatchOutput) -> Value {
    match output {
        DispatchOutput::Ack => json!({ "kind": "ack" }),
        DispatchOutput::CpuStep { trace, cpu_cycles } => {
            json!({ "kind": "cpu_step", "trace": trace, "cpu_cycles": cpu_cycles })
        }
        DispatchOutput::CycleCount { cpu_cycles } => {
            json!({ "kind": "cycle_count", "cpu_cycles": cpu_cycles })
        }
        DispatchOutput::ControllerState { controller_bits } => {
            json!({ "kind": "controller_state", "controller_bits": controller_bits })
        }
        DispatchOutput::EmulatorState {
            paused,
            speed_permille,
            controller_bits,
        } => {
            json!({
                "kind": "emulator_state",
                "paused": paused,
                "speed_permille": speed_permille,
                "controller_bits": controller_bits
            })
        }
        DispatchOutput::Registers {
            pc,
            a,
            x,
            y,
            sp,
            status,
        } => {
            json!({
                "kind": "registers",
                "pc": pc,
                "a": a,
                "x": x,
                "y": y,
                "sp": sp,
                "status": status
            })
        }
        DispatchOutput::Memory { address, value } => {
            json!({ "kind": "memory", "address": address, "value": value })
        }
        DispatchOutput::Fps { fps_milli } => json!({ "kind": "fps", "fps_milli": fps_milli }),
        DispatchOutput::PpuFrameCounter { frame_counter } => {
            json!({ "kind": "ppu_frame_counter", "frame_counter": frame_counter })
        }
        DispatchOutput::Frame { seq, bytes } => {
            json!({ "kind": "frame", "seq": seq, "bytes": bytes })
        }
        DispatchOutput::FrameCaptured { path, bytes } => {
            json!({ "kind": "frame_captured", "path": path, "bytes": bytes })
        }
        DispatchOutput::Audio { seq, samples } => {
            json!({ "kind": "audio", "seq": seq, "samples": samples })
        }
        DispatchOutput::StateSlot { slot } => {
            json!({ "kind": "state_slot", "slot": slot })
        }
        DispatchOutput::RomLoaded {
            mapper_id,
            prg_rom_bytes,
            reset_pc,
        } => {
            json!({
                "kind": "rom_loaded",
                "mapper_id": mapper_id,
                "prg_rom_bytes": prg_rom_bytes,
                "reset_pc": reset_pc
            })
        }
        DispatchOutput::MacroExecuted {
            frames_elapsed,
            final_controller_bits,
        } => {
            json!({
                "kind": "macro_executed",
                "frames_elapsed": frames_elapsed,
                "final_controller_bits": final_controller_bits,
            })
        }
        DispatchOutput::DslAssembled {
            bytes_written,
            label_count,
            nmi_vector,
            reset_vector,
            irq_vector,
        } => {
            json!({
                "kind": "dsl_assembled",
                "bytes_written": bytes_written,
                "label_count": label_count,
                "nmi_vector": nmi_vector,
                "reset_vector": reset_vector,
                "irq_vector": irq_vector,
            })
        }
        DispatchOutput::DslRomLoaded {
            mapper_id,
            prg_rom_bytes,
            reset_pc,
            rom_bytes,
        } => {
            json!({
                "kind": "dsl_rom_loaded",
                "mapper_id": mapper_id,
                "prg_rom_bytes": prg_rom_bytes,
                "reset_pc": reset_pc,
                "rom_bytes": rom_bytes,
            })
        }
        DispatchOutput::DslRomExported {
            path,
            bytes,
            mapper_id,
            prg_rom_bytes,
        } => {
            json!({
                "kind": "dsl_rom_exported",
                "path": path,
                "bytes": bytes,
                "mapper_id": mapper_id,
                "prg_rom_bytes": prg_rom_bytes,
            })
        }
        DispatchOutput::DslRomExportedBase64 {
            rom_base64,
            bytes,
            mapper_id,
            prg_rom_bytes,
        } => {
            json!({
                "kind": "dsl_rom_exported_base64",
                "rom_base64": rom_base64,
                "bytes": bytes,
                "mapper_id": mapper_id,
                "prg_rom_bytes": prg_rom_bytes,
            })
        }
    }
}

fn tool_input_schema(tool_name: &str) -> Value {
    match tool_name {
        "set_controller_state" => json!({
            "type": "object",
            "properties": {
                "bits": { "type": "integer", "minimum": 0, "maximum": 255 },
                "player": { "type": "integer", "enum": [1, 2] }
            },
            "required": ["bits"],
            "additionalProperties": false
        }),
        "press_button" | "release_button" => json!({
            "type": "object",
            "properties": {
                "button": {
                    "type": "string",
                    "enum": ["A", "B", "Select", "Start", "Up", "Down", "Left", "Right"]
                },
                "player": { "type": "integer", "enum": [1, 2] }
            },
            "required": ["button"],
            "additionalProperties": false
        }),
        "set_speed" => json!({
            "type": "object",
            "properties": {
                "multiplier": { "type": "number", "exclusiveMinimum": 0.0 }
            },
            "required": ["multiplier"],
            "additionalProperties": false
        }),
        "read_memory" | "set_breakpoint" | "clear_breakpoint" | "disassemble_at" => json!({
            "type": "object",
            "properties": {
                "address": { "type": "integer", "minimum": 0, "maximum": 65535 }
            },
            "required": ["address"],
            "additionalProperties": false
        }),
        "get_frame" | "get_audio_chunk" => json!({
            "type": "object",
            "properties": {
                "seq": { "type": "integer", "minimum": 0 }
            },
            "additionalProperties": false
        }),
        "capture_frame" => json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "minLength": 1 }
            },
            "required": ["path"],
            "additionalProperties": false
        }),
        "save_state" | "load_state" => json!({
            "type": "object",
            "properties": {
                "slot": { "type": "string", "minLength": 1 }
            },
            "additionalProperties": false
        }),
        "load_rom" => json!({
            "type": "object",
            "properties": {
                "rom_path": { "type": "string", "minLength": 1 },
                "rom_hex": { "type": "string", "minLength": 2 }
            },
            "oneOf": [
                { "required": ["rom_path"] },
                { "required": ["rom_hex"] }
            ],
            "additionalProperties": false
        }),
        "assemble_6502_dsl" => json!({
            "type": "object",
            "properties": {
                "source": { "type": "string", "minLength": 1 }
            },
            "required": ["source"],
            "additionalProperties": false
        }),
        "load_6502_dsl" => json!({
            "type": "object",
            "properties": {
                "source": { "type": "string", "minLength": 1 },
                "mirroring": { "type": "string", "enum": ["horizontal", "vertical"] },
                "chr_hex": { "type": "string", "minLength": 2 }
            },
            "required": ["source"],
            "additionalProperties": false
        }),
        "export_6502_dsl_rom" => json!({
            "type": "object",
            "properties": {
                "source": { "type": "string", "minLength": 1 },
                "output_path": { "type": "string", "minLength": 1 },
                "mirroring": { "type": "string", "enum": ["horizontal", "vertical"] },
                "chr_hex": { "type": "string", "minLength": 2 }
            },
            "required": ["source", "output_path"],
            "additionalProperties": false
        }),
        "export_6502_dsl_rom_base64" => json!({
            "type": "object",
            "properties": {
                "source": { "type": "string", "minLength": 1 },
                "mirroring": { "type": "string", "enum": ["horizontal", "vertical"] },
                "chr_hex": { "type": "string", "minLength": 2 }
            },
            "required": ["source"],
            "additionalProperties": false
        }),
        _ => json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
    }
}

fn read_stdio_message(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, McpError> {
    let mut content_length = None::<usize>;
    loop {
        let mut line = String::new();
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

fn write_stdio_message(writer: &mut impl Write, value: &Value) -> Result<(), McpError> {
    let payload = serde_json::to_vec(value)
        .map_err(|err| McpError::Protocol(format!("failed serializing JSON response: {err}")))?;
    writer
        .write_all(format!("Content-Length: {}\r\n\r\n", payload.len()).as_bytes())
        .and_then(|_| writer.write_all(&payload))
        .and_then(|_| writer.flush())
        .map_err(|err| McpError::Protocol(format!("failed writing stdio response: {err}")))
}

fn jsonrpc_result_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "result": result
    })
}

fn jsonrpc_error_response(id: Value, err: RpcError) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "error": err.to_json()
    })
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

        assert_eq!(parse.to_json()["message"], json!("bad json"));
        assert_eq!(invalid.to_json()["message"], json!("bad request"));
        assert_eq!(missing.to_json()["message"], json!("missing"));
        assert_eq!(params.to_json()["message"], json!("bad params"));
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
    fn map_tool_arguments_stringifies_supported_json_types() {
        let mut arguments = Map::new();
        arguments.insert("str".to_owned(), json!("value"));
        arguments.insert("num".to_owned(), json!(123));
        arguments.insert("bool".to_owned(), json!(true));
        arguments.insert("null".to_owned(), Value::Null);
        arguments.insert("object".to_owned(), json!({ "x": 1 }));

        let mapped = map_tool_arguments(arguments);
        assert_eq!(mapped.get("str"), Some(&"value".to_owned()));
        assert_eq!(mapped.get("num"), Some(&"123".to_owned()));
        assert_eq!(mapped.get("bool"), Some(&"true".to_owned()));
        assert_eq!(mapped.get("null"), Some(&"null".to_owned()));
        assert_eq!(mapped.get("object"), Some(&"{\"x\":1}".to_owned()));
    }

    #[test]
    fn tool_input_schema_covers_controller_memory_and_dsl_groups() {
        let controller = tool_input_schema("set_controller_state");
        assert_eq!(controller["required"], json!(["bits"]));
        assert!(controller["properties"]["player"].is_object());

        let buttons = tool_input_schema("press_button");
        assert_eq!(buttons["required"], json!(["button"]));
        assert!(buttons["properties"]["button"]["enum"].is_array());

        let speed = tool_input_schema("set_speed");
        assert_eq!(speed["required"], json!(["multiplier"]));

        let memory = tool_input_schema("read_memory");
        assert_eq!(memory["required"], json!(["address"]));

        let frame = tool_input_schema("get_frame");
        assert!(frame["properties"]["seq"].is_object());

        let capture = tool_input_schema("capture_frame");
        assert_eq!(capture["required"], json!(["path"]));

        let slots = tool_input_schema("save_state");
        assert!(slots["properties"]["slot"].is_object());

        let assemble = tool_input_schema("assemble_6502_dsl");
        assert_eq!(assemble["required"], json!(["source"]));

        let load_dsl = tool_input_schema("load_6502_dsl");
        assert!(load_dsl["properties"]["mirroring"]["enum"].is_array());
    }

    #[test]
    fn jsonrpc_error_response_wraps_error_payload() {
        let response = jsonrpc_error_response(json!(99), RpcError::invalid_params("bad input"));
        assert_eq!(response["jsonrpc"], json!("2.0"));
        assert_eq!(response["id"], json!(99));
        assert_eq!(response["error"]["code"], json!(-32602));
        assert_eq!(response["error"]["message"], json!("bad input"));
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
