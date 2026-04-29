//! Model Context Protocol (MCP) host server integration.
//!
//! This module provides the `McpHost`, a background server that bridges the
//! emulator core to an external MCP client via JSON-RPC over TCP.
//!
//! # Architecture
//! The MCP host runs its TCP listener and connection handling on a dedicated
//! background thread. Because the `NesCore` emulator state is thread-local and
//! must be tightly synchronized with the render loop, the background thread
//! does not execute emulator commands directly. Instead, it parses incoming
//! requests and forwards them via an `mpsc` channel to the main thread.
//! The main thread must regularly call `McpHost::drain` to apply these queued
//! commands to the emulator state.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use nes_core::NesCore;
use nes_mcp::{
    DispatchError, DispatchOutput, ToolParams, dispatch_tool,
    protocol::{
        DEFAULT_PROTOCOL_VERSION, JSONRPC_VERSION, RpcError, RpcRequest, dispatch_output_value,
        jsonrpc_error, jsonrpc_result, map_tool_arguments, tool_input_schema,
    },
    tool_catalog,
};
use serde_json::{Value, json};

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
    /// Starts the MCP background server, listening for incoming JSON-RPC connections.
    ///
    /// This method binds a TCP listener to the specified address and spawns a background thread
    /// to handle client requests. The server accepts commands like reading memory or controlling
    /// the emulator state, which are sent back to the main thread via a channel.
    ///
    /// # Errors
    ///
    /// Returns a `String` containing the error message if the server fails to bind to the specified address.
    ///
    /// # Examples
    ///
    /// ```
    /// use nes_desktop::mcp_host::McpHost;
    ///
    /// // Start the server on an ephemeral port
    /// let host = McpHost::start("127.0.0.1:0").unwrap();
    /// println!("Bound to {}", host.bind_addr());
    /// ```
    pub fn start(bind_addr: &str) -> Result<Self, String> {
        let listener = TcpListener::bind(bind_addr)
            .map_err(|err| format!("Failed to bind MCP host at '{bind_addr}': {err}"))?;
        let local_addr = listener
            .local_addr()
            .map_err(|err| format!("Failed to read MCP host local addr: {err}"))?
            .to_string();

        let (tx, rx) = mpsc::channel::<ToolRequest>();
        let thread_bind_addr = local_addr.clone();
        let handle = thread::spawn(move || run_listener(listener, tx, &thread_bind_addr));

        Ok(Self {
            requests: rx,
            _thread: handle,
            bind_addr: local_addr,
        })
    }

    /// Returns the local address that the MCP server is currently bound to.
    ///
    /// This is particularly useful when starting the server with port `0` (an ephemeral port),
    /// as it allows the host to query the operating system for the actual port that was assigned.
    ///
    /// # Examples
    ///
    /// ```
    /// use nes_desktop::mcp_host::McpHost;
    ///
    /// let host = McpHost::start("127.0.0.1:0").unwrap();
    /// let addr = host.bind_addr();
    /// assert!(addr.starts_with("127.0.0.1:"));
    /// ```
    pub fn bind_addr(&self) -> &str {
        &self.bind_addr
    }

    /// Drains all pending MCP requests and executes them against the emulator core.
    ///
    /// The MCP server runs on a background thread and sends requests to the main thread via a channel.
    /// The main thread (running the emulator) must call this method regularly (e.g., once per frame)
    /// to process and reply to these requests.
    ///
    /// If an empty or uninitialized [`NesCore`] is provided, tools like reading memory or injecting
    /// inputs may fail gracefully depending on their internal error handling.
    ///
    /// # Examples
    ///
    /// ```
    /// use nes_desktop::mcp_host::McpHost;
    /// use nes_core::NesCore;
    ///
    /// let host = McpHost::start("127.0.0.1:0").unwrap();
    /// let mut core = NesCore::new();
    ///
    /// // Typically called inside the main emulation loop
    /// host.drain(&mut core);
    /// ```
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
                let tx_clone = request_tx.clone();
                let addr_clone = bind_addr.to_owned();
                thread::spawn(move || {
                    if let Err(err) = handle_client(stream, &tx_clone) {
                        eprintln!("[mcp-host {addr_clone}] client error: {err}");
                    }
                });
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
    let Some(Value::Object(mut params_obj)) = params else {
        return Err(RpcError::invalid_params(
            "tools/call params must be an object",
        ));
    };
    let Some(Value::String(tool_name)) = params_obj.remove("name") else {
        return Err(RpcError::invalid_params(
            "tools/call requires string field 'name'",
        ));
    };
    let args = if let Some(Value::Object(map)) = params_obj.remove("arguments") {
        map
    } else {
        Default::default()
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

/// Reads an MCP framed message from the given buffered reader stream.
///
/// This reads standard MCP headers (e.g. `Content-Length`), followed by an empty line,
/// and then reads the exact number of bytes specified by the length into a buffer.
/// If EOF is reached before a message is found, it returns `Ok(None)`.
///
/// # Examples
///
/// ```
/// # use nes_desktop::mcp_host::read_framed_message;
/// # use std::io::Cursor;
/// let data = b"Content-Length: 13\r\n\r\n{\"key\":\"val\"}";
/// let mut reader = Cursor::new(data);
/// let result = read_framed_message(&mut reader).unwrap();
/// assert_eq!(result.unwrap(), b"{\"key\":\"val\"}");
/// ```
pub fn read_framed_message(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, String> {
    let mut content_length = None::<usize>;
    let mut line = String::new();

    loop {
        line.clear();

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

    const MAX_PAYLOAD_SIZE: usize = 10 * 1024 * 1024; // 10 MB
    if len > MAX_PAYLOAD_SIZE {
        return Err(format!(
            "Content-Length {len} exceeds maximum allowed size of {MAX_PAYLOAD_SIZE} bytes"
        ));
    }

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
            || lowered.contains("os error 10060")
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
    fn host_start_fails_on_invalid_bind_address() {
        let result = McpHost::start("256.256.256.256:0");
        if let Err(err) = result {
            assert!(err.contains("Failed to bind MCP host"));
        } else {
            panic!("Expected an error when binding to invalid address");
        }
    }
}

#[cfg(test)]
mod coverage_tests {
    use super::*;
    #[test]
    fn test_mcp_host_start() {
        let host = McpHost::start("127.0.0.1:0").unwrap();
        assert!(host.bind_addr().starts_with("127.0.0.1:"));
    }
}
