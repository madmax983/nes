use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use nes_core::NesCore;
use nes_mcp::{DispatchError, DispatchOutput, ToolParams, dispatch_tool, tool_catalog};
use serde::Deserialize;
use serde_json::{Map, Value, json};

const JSONRPC_VERSION: &str = "2.0";
const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";
const TOOL_CALL_TIMEOUT: Duration = Duration::from_secs(5);

pub struct McpHost {
    requests: Receiver<ToolRequest>,
    _thread: thread::JoinHandle<()>,
    bind_addr: String,
}

struct ToolRequest {
    name: String,
    params: ToolParams,
    respond_to: Sender<Result<DispatchOutput, DispatchError>>,
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
}

impl RpcError {
    fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: message.into(),
        }
    }

    fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
        }
    }

    fn method_not_found(message: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: message.into(),
        }
    }

    fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
        }
    }

    fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
        }
    }

    fn as_json(&self) -> Value {
        json!({
            "code": self.code,
            "message": self.message
        })
    }
}

impl McpHost {
    pub fn start(bind_addr: &str) -> Result<Self, String> {
        let listener = TcpListener::bind(bind_addr)
            .map_err(|err| format!("Failed to bind MCP host at '{bind_addr}': {err}"))?;
        let local_addr = listener
            .local_addr()
            .map_err(|err| format!("Failed to read MCP host local addr: {err}"))?
            .to_string();

        let (tx, rx) = mpsc::channel::<ToolRequest>();
        let thread_tx = tx.clone();
        let thread_bind_addr = local_addr.clone();
        let handle = thread::spawn(move || run_listener(listener, thread_tx, &thread_bind_addr));

        Ok(Self {
            requests: rx,
            _thread: handle,
            bind_addr: local_addr,
        })
    }

    pub fn bind_addr(&self) -> &str {
        &self.bind_addr
    }

    pub fn drain(&self, core: &mut NesCore) {
        while let Ok(request) = self.requests.try_recv() {
            let result = dispatch_tool(core, &request.name, &request.params);
            let _ = request.respond_to.send(result);
        }
    }
}

fn run_listener(listener: TcpListener, request_tx: Sender<ToolRequest>, bind_addr: &str) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(err) = handle_client(stream, &request_tx) {
                    eprintln!("[mcp-host {bind_addr}] client error: {err}");
                }
            }
            Err(err) => eprintln!("[mcp-host {bind_addr}] accept failed: {err}"),
        }
    }
}

fn handle_client(mut stream: TcpStream, request_tx: &Sender<ToolRequest>) -> Result<(), String> {
    let mut reader = BufReader::new(
        stream
            .try_clone()
            .map_err(|err| format!("failed to clone client socket: {err}"))?,
    );

    loop {
        let Some(payload) = read_framed_message(&mut reader)? else {
            break;
        };
        let Some(response) = handle_message(&payload, request_tx) else {
            continue;
        };
        write_framed_message(&mut stream, &response)?;
    }
    Ok(())
}

fn handle_message(payload: &[u8], request_tx: &Sender<ToolRequest>) -> Option<Value> {
    let request: RpcRequest = match serde_json::from_slice(payload) {
        Ok(request) => request,
        Err(err) => {
            return Some(jsonrpc_error(
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
        return Some(jsonrpc_error(
            response_id,
            RpcError::invalid_request("jsonrpc must be '2.0'"),
        ));
    }

    let method_result = match request.method.as_str() {
        "initialize" => handle_initialize(request.params.as_ref()),
        "notifications/initialized" => Ok(None),
        "ping" => Ok(Some(json!({}))),
        "tools/list" => Ok(Some(handle_tools_list())),
        "tools/call" => handle_tools_call(request.params.as_ref(), request_tx),
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

fn handle_initialize(params: Option<&Value>) -> Result<Option<Value>, RpcError> {
    let params_obj = params
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError::invalid_params("initialize params must be an object"))?;

    let requested_protocol = params_obj
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_PROTOCOL_VERSION);

    Ok(Some(json!({
        "protocolVersion": requested_protocol,
        "capabilities": {
            "tools": {
                "listChanged": false
            }
        },
        "serverInfo": {
            "name": "nes-desktop-mcp-host",
            "version": env!("CARGO_PKG_VERSION")
        },
        "instructions": "Embedded MCP host for the live desktop emulator core."
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
    params: Option<&Value>,
    request_tx: &Sender<ToolRequest>,
) -> Result<Option<Value>, RpcError> {
    let params_obj = params
        .and_then(Value::as_object)
        .ok_or_else(|| RpcError::invalid_params("tools/call params must be an object"))?;
    let tool_name = params_obj
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| RpcError::invalid_params("tools/call requires string field 'name'"))?;
    let args = params_obj
        .get("arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let (reply_tx, reply_rx) = mpsc::channel();
    request_tx
        .send(ToolRequest {
            name: tool_name.to_owned(),
            params: map_tool_arguments(&args),
            respond_to: reply_tx,
        })
        .map_err(|_| RpcError::internal_error("desktop core request channel closed"))?;

    let call_result = reply_rx
        .recv_timeout(TOOL_CALL_TIMEOUT)
        .map_err(|_| RpcError::internal_error("tool call timed out waiting for desktop core"))?;

    let response = match call_result {
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

    Ok(Some(response))
}

fn map_tool_arguments(arguments: &Map<String, Value>) -> ToolParams {
    let mut params = ToolParams::new();
    for (key, value) in arguments {
        params.insert(key.clone(), json_arg_to_string(value));
    }
    params
}

fn json_arg_to_string(value: &Value) -> String {
    match value {
        Value::String(v) => v.clone(),
        Value::Number(v) => v.to_string(),
        Value::Bool(v) => v.to_string(),
        Value::Null => "null".to_owned(),
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
    }
}

fn tool_input_schema(tool_name: &str) -> Value {
    match tool_name {
        "set_controller_state" => json!({
            "type": "object",
            "properties": {
                "bits": { "type": "integer", "minimum": 0, "maximum": 255 }
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
                }
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
        _ => json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        }),
    }
}

fn read_framed_message(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, String> {
    let mut content_length = None::<usize>;
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|err| format!("failed reading header line: {err}"))?;
        if read == 0 {
            if content_length.is_none() {
                return Ok(None);
            }
            return Err("unexpected EOF while reading MCP headers".to_owned());
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            let parsed = value.trim();
            let len = parsed
                .parse::<usize>()
                .map_err(|_| format!("invalid Content-Length value '{parsed}'"))?;
            content_length = Some(len);
        }
    }

    let len = content_length.ok_or_else(|| "missing Content-Length header".to_owned())?;
    let mut payload = vec![0_u8; len];
    reader
        .read_exact(&mut payload)
        .map_err(|err| format!("failed reading payload body: {err}"))?;
    Ok(Some(payload))
}

fn write_framed_message(writer: &mut impl Write, value: &Value) -> Result<(), String> {
    let payload = serde_json::to_vec(value)
        .map_err(|err| format!("failed serializing JSON response: {err}"))?;
    writer
        .write_all(format!("Content-Length: {}\r\n\r\n", payload.len()).as_bytes())
        .and_then(|_| writer.write_all(&payload))
        .and_then(|_| writer.flush())
        .map_err(|err| format!("failed writing framed response: {err}"))
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "result": result
    })
}

fn jsonrpc_error(id: Value, err: RpcError) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "error": err.as_json()
    })
}
