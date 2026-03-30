use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use nes_core::NesCore;
use nes_mcp::{
    DispatchError, DispatchOutput, ToolParams, dispatch_tool,
    protocol::{RpcError, RpcRequest},
    tool_catalog,
};
use serde_json::{Map, Value, json};

const JSONRPC_VERSION: &str = "2.0";
const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";
const TOOL_CALL_TIMEOUT: Duration = Duration::from_secs(5);

/// A background server that bridges the emulator core to the Model Context Protocol (MCP).
///
/// `McpHost` listens on a local TCP socket for incoming JSON-RPC connections from
/// an MCP client. It delegates commands like reading memory or injecting inputs
/// to the actual emulator running on the main thread.
///
/// ## Examples
///
/// ```
/// use nes_desktop::mcp_host::McpHost;
/// use nes_core::NesCore;
///
/// // Start the server on an ephemeral port
/// let host = McpHost::start("127.0.0.1:0").unwrap();
/// println!("Listening on {}", host.bind_addr());
///
/// // The emulator core must regularly drain requests
/// let mut core = NesCore::new();
/// host.drain(&mut core);
/// ```
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

    while let Some(payload) = read_framed_message(&mut reader)? {
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
        "tools/call" => handle_tools_call(request.params, request_tx),
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
    params: Option<Value>,
    request_tx: &Sender<ToolRequest>,
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

    let (reply_tx, reply_rx) = mpsc::channel();
    request_tx
        .send(ToolRequest {
            name: tool_name.to_owned(),
            params: map_tool_arguments(args),
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
        DispatchOutput::PpuOam { oam_bytes } => {
            json!({
                "kind": "ppu_oam",
                "oam_bytes": oam_bytes,
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

/// Writes a framed JSON-RPC message to the provided writer.
///
/// **Performance optimization:** Uses `write!` macro directly to the writer instead
/// of allocating an intermediate `String` via `format!` to construct the `Content-Length` header.
fn write_framed_message(writer: &mut impl Write, value: &Value) -> Result<(), String> {
    let payload = serde_json::to_vec(value)
        .map_err(|err| format!("failed serializing JSON response: {err}"))?;
    write!(writer, "Content-Length: {}\r\n\r\n", payload.len())
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
        "error": err.to_json()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nes_core::NesCore;
    use std::io::{BufReader, Cursor};
    use std::net::TcpStream;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    fn is_retryable_read_error(message: &str) -> bool {
        let lowered = message.to_ascii_lowercase();
        lowered.contains("temporarily unavailable")
            || lowered.contains("would block")
            || lowered.contains("timed out")
            || lowered.contains("non-blocking socket operation")
    }

    fn read_response_with_host_drain(
        host: &McpHost,
        core: &mut NesCore,
        reader: &mut BufReader<TcpStream>,
        context: &str,
    ) -> Vec<u8> {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            host.drain(core);
            match read_framed_message(reader) {
                Ok(Some(payload)) => return payload,
                Ok(None) => panic!("{context}: connection closed before response"),
                Err(err) if Instant::now() < deadline && is_retryable_read_error(&err) => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => panic!("{context}: {err}"),
            }
        }
    }

    #[test]
    fn rpc_error_constructors_use_jsonrpc_standard_codes() {
        let parse = RpcError::parse_error("bad json");
        assert_eq!(parse.code, -32700);
        assert_eq!(parse.message, "bad json");

        let invalid_request = RpcError::invalid_request("missing id");
        assert_eq!(invalid_request.code, -32600);
        assert_eq!(invalid_request.message, "missing id");

        let method_not_found = RpcError::method_not_found("unknown method");
        assert_eq!(method_not_found.code, -32601);
        assert_eq!(method_not_found.message, "unknown method");

        let invalid_params = RpcError::invalid_params("bad args");
        assert_eq!(invalid_params.code, -32602);
        assert_eq!(invalid_params.message, "bad args");

        let internal = RpcError::internal_error("boom");
        assert_eq!(internal.code, -32603);
        assert_eq!(internal.message, "boom");

        let json = internal.to_json();
        assert_eq!(json["code"], -32603);
        assert_eq!(json["message"], "boom");
    }

    #[test]
    fn jsonrpc_helper_envelopes_include_version_ids_and_payloads() {
        let result = jsonrpc_result(json!(7), json!({"ok": true}));
        assert_eq!(result["jsonrpc"], JSONRPC_VERSION);
        assert_eq!(result["id"], 7);
        assert_eq!(result["result"]["ok"], true);

        let error = jsonrpc_error(json!("abc"), RpcError::invalid_request("bad request"));
        assert_eq!(error["jsonrpc"], JSONRPC_VERSION);
        assert_eq!(error["id"], "abc");
        assert_eq!(error["error"]["code"], -32600);
        assert_eq!(error["error"]["message"], "bad request");
    }

    #[test]
    fn json_arg_and_tool_argument_mapping_stringify_scalars_and_structures() {
        assert_eq!(json_arg_to_string(json!("x")), "x");
        assert_eq!(json_arg_to_string(json!(42)), "42");
        assert_eq!(json_arg_to_string(json!(true)), "true");
        assert_eq!(json_arg_to_string(Value::Null), "null");
        assert_eq!(json_arg_to_string(json!({"k": "v"})), "{\"k\":\"v\"}");

        let mut args = Map::<String, Value>::new();
        args.insert("a".to_owned(), json!(5));
        args.insert("b".to_owned(), json!(false));
        let mapped = map_tool_arguments(args);
        assert_eq!(mapped.get("a").map(String::as_str), Some("5"));
        assert_eq!(mapped.get("b").map(String::as_str), Some("false"));
    }

    #[test]
    fn handle_initialize_and_message_routes_cover_core_jsonrpc_paths() {
        let init_err = handle_initialize(None).expect_err("missing params should fail");
        assert_eq!(init_err.code, -32602);

        let init = handle_initialize(Some(&json!({"protocolVersion": "2025-01-01"})))
            .expect("valid initialize should succeed")
            .expect("initialize should return a response");
        assert_eq!(init["protocolVersion"], "2025-01-01");
        assert_eq!(init["serverInfo"]["name"], "nes-desktop-mcp-host");

        let (request_tx, _request_rx) = mpsc::channel::<ToolRequest>();
        let parse = handle_message(b"{", &request_tx).expect("parse errors should respond");
        assert_eq!(parse["error"]["code"], -32700);

        let wrong_version_payload = serde_json::to_vec(&json!({
            "jsonrpc": "1.0",
            "id": 1,
            "method": "ping"
        }))
        .expect("serialize wrong version request");
        let wrong_version = handle_message(&wrong_version_payload, &request_tx)
            .expect("invalid version should respond");
        assert_eq!(wrong_version["error"]["code"], -32600);

        let ping_payload = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "ping"
        }))
        .expect("serialize ping request");
        let ping = handle_message(&ping_payload, &request_tx).expect("ping should respond");
        assert_eq!(ping["result"], json!({}));

        let init_payload = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "initialize",
            "params": { "protocolVersion": "2025-06-18" }
        }))
        .expect("serialize initialize request");
        let init_response =
            handle_message(&init_payload, &request_tx).expect("initialize should respond");
        assert_eq!(init_response["id"], 10);
        assert_eq!(init_response["result"]["protocolVersion"], "2025-06-18");

        let initialized_payload = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "notifications/initialized"
        }))
        .expect("serialize initialized notification");
        let initialized_response = handle_message(&initialized_payload, &request_tx)
            .expect("initialized notification with id should respond");
        assert_eq!(initialized_response["result"], json!({}));

        let resources_payload = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "resources/list"
        }))
        .expect("serialize resources/list request");
        let resources_response =
            handle_message(&resources_payload, &request_tx).expect("resources/list should respond");
        assert!(resources_response["result"]["resources"].is_array());

        let prompts_payload = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "prompts/list"
        }))
        .expect("serialize prompts/list request");
        let prompts_response =
            handle_message(&prompts_payload, &request_tx).expect("prompts/list should respond");
        assert!(prompts_response["result"]["prompts"].is_array());

        let logging_payload = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "id": 14,
            "method": "logging/setLevel"
        }))
        .expect("serialize logging request");
        let logging_response =
            handle_message(&logging_payload, &request_tx).expect("logging request should respond");
        assert_eq!(logging_response["result"], json!({}));

        let notification_payload = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))
        .expect("serialize notification");
        assert_eq!(handle_message(&notification_payload, &request_tx), None);
    }

    #[test]
    fn handle_tools_call_dispatches_requests_and_wraps_dispatch_output() {
        let (request_tx, request_rx) = mpsc::channel::<ToolRequest>();
        let worker = std::thread::spawn(move || {
            let req = request_rx.recv().expect("request should be delivered");
            assert_eq!(req.name, "pause");
            let _ = req.respond_to.send(Ok(DispatchOutput::Ack));
        });

        let params = json!({
            "name": "pause",
            "arguments": {
                "speed": 2
            }
        });
        let response = handle_tools_call(Some(params), &request_tx)
            .expect("tools/call should succeed")
            .expect("tools/call should return payload");
        assert_eq!(response["isError"], false);
        assert_eq!(response["structuredContent"]["kind"], "ack");
        worker.join().expect("worker join should succeed");

        let missing_name = handle_tools_call(Some(json!({ "arguments": {} })), &request_tx)
            .expect_err("missing name should fail");
        assert_eq!(missing_name.code, -32602);
    }

    #[test]
    fn framed_message_io_round_trips_payload_and_validates_headers() {
        let value = json!({"kind":"ping","nonce":7});
        let mut wire = Vec::<u8>::new();
        write_framed_message(&mut wire, &value).expect("framed write should succeed");

        let mut reader = BufReader::new(Cursor::new(wire));
        let payload = read_framed_message(&mut reader)
            .expect("framed read should succeed")
            .expect("payload should exist");
        let parsed: Value = serde_json::from_slice(&payload).expect("payload JSON should decode");
        assert_eq!(parsed, value);

        let mut bad_reader = BufReader::new(Cursor::new(b"{}\r\n\r\n".to_vec()));
        let err =
            read_framed_message(&mut bad_reader).expect_err("missing Content-Length should fail");
        assert!(err.contains("missing Content-Length"));
    }

    #[test]
    fn tool_input_schema_covers_all_specialized_method_shapes() {
        let methods_with_property = [
            ("set_controller_state", "bits"),
            ("press_button", "button"),
            ("release_button", "button"),
            ("set_speed", "multiplier"),
            ("read_memory", "address"),
            ("set_breakpoint", "address"),
            ("clear_breakpoint", "address"),
            ("disassemble_at", "address"),
            ("get_frame", "seq"),
            ("get_audio_chunk", "seq"),
            ("capture_frame", "path"),
            ("save_state", "slot"),
            ("load_state", "slot"),
            ("load_rom", "rom_path"),
            ("assemble_6502_dsl", "source"),
            ("load_6502_dsl", "source"),
            ("export_6502_dsl_rom", "output_path"),
            ("export_6502_dsl_rom_base64", "source"),
        ];

        for (method, property) in methods_with_property {
            let schema = tool_input_schema(method);
            assert_eq!(schema["type"], "object", "schema type for {method}");
            assert!(schema["properties"][property].is_object());
        }
        let fallback = tool_input_schema("unknown");
        assert_eq!(fallback["type"], "object");
        assert!(fallback["properties"].is_object());
    }

    #[test]
    fn host_start_and_client_round_trip_cover_bind_addr_drain_and_dispatch() {
        let host = McpHost::start("127.0.0.1:0").expect("host should start");
        let bind_addr = host.bind_addr().to_owned();
        assert!(
            bind_addr.starts_with("127.0.0.1:"),
            "bind addr should include localhost endpoint"
        );

        let mut stream = TcpStream::connect(&bind_addr).expect("client should connect to host");
        stream
            .set_read_timeout(Some(Duration::from_millis(500)))
            .expect("set read timeout");
        let mut reader = BufReader::new(
            stream
                .try_clone()
                .expect("client stream clone for reader should succeed"),
        );

        let ping_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping"
        });
        write_framed_message(&mut stream, &ping_request).expect("write ping");
        let ping_payload = read_framed_message(&mut reader)
            .expect("read ping response")
            .expect("ping payload");
        let ping_response: Value =
            serde_json::from_slice(&ping_payload).expect("decode ping response");
        assert_eq!(ping_response["id"], 1);
        assert_eq!(ping_response["result"], json!({}));

        let tools_list_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });
        write_framed_message(&mut stream, &tools_list_request).expect("write tools/list");
        let list_payload = read_framed_message(&mut reader)
            .expect("read tools/list response")
            .expect("tools/list payload");
        let list_response: Value =
            serde_json::from_slice(&list_payload).expect("decode tools/list response");
        assert!(list_response["result"]["tools"].is_array());

        let tools_call_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "pause",
                "arguments": {}
            }
        });
        write_framed_message(&mut stream, &tools_call_request).expect("write tools/call");

        let mut core = NesCore::new();
        let call_payload = read_response_with_host_drain(
            &host,
            &mut core,
            &mut reader,
            "read tools/call response",
        );
        let call_response: Value =
            serde_json::from_slice(&call_payload).expect("decode tools/call response");
        assert_eq!(call_response["id"], 3);
        assert_eq!(call_response["result"]["isError"], false);
        assert_eq!(call_response["result"]["structuredContent"]["kind"], "ack");
    }


    #[test]
    #[should_panic(expected = "capacity overflow")]
    #[ignore = "havoc target"]
    fn havoc_crash_mcp_host_oom() {
        let mut bad_reader = BufReader::new(Cursor::new(b"Content-Length: 18446744073709551615\r\n\r\n".to_vec()));
        let _ = read_framed_message(&mut bad_reader);
    }
}
