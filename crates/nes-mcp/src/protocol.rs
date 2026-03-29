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
use serde_json::{Map, Value, json};

use crate::{DispatchOutput, ToolParams};

pub const JSONRPC_VERSION: &str = "2.0";
pub const DEFAULT_PROTOCOL_VERSION: &str = "2025-06-18";

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

/// Maps raw JSON RPC arguments into a strongly typed `ToolParams` map.
///
/// **Performance optimization:** This function consumes an owned `Map<String, Value>`
/// rather than taking a borrowed reference `&Map`. Because the MCP dispatch
/// layer typically already has an owned arguments object, taking ownership here
/// entirely eliminates the need to allocate and `.clone()` every string key
/// and value when inserting them into the returned `ToolParams`.
pub fn map_tool_arguments(arguments: Map<String, Value>) -> ToolParams {
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
pub fn json_arg_to_string(value: Value) -> String {
    match value {
        Value::String(v) => v,
        _ => value.to_string(),
    }
}

pub fn dispatch_output_value(output: DispatchOutput) -> Value {
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
        DispatchOutput::PpuOam { oam_bytes } => {
            json!({
                "kind": "ppu_oam",
                "oam_bytes": oam_bytes,
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

pub fn tool_input_schema(tool_name: &str) -> Value {
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

pub fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "result": result
    })
}

pub fn jsonrpc_error(id: Value, err: RpcError) -> Value {
    json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": id,
        "error": err.to_json()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn jsonrpc_error_wraps_error_payload() {
        let response = jsonrpc_error(json!(99), RpcError::invalid_params("bad input"));
        assert_eq!(response["jsonrpc"], json!("2.0"));
        assert_eq!(response["id"], json!(99));
        assert_eq!(response["error"]["code"], json!(-32602));
        assert_eq!(response["error"]["message"], json!("bad input"));
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
    fn dispatch_output_value_maps_variants_to_json() {
        let cases = vec![
            (DispatchOutput::Ack, json!({ "kind": "ack" })),
            (
                DispatchOutput::CpuStep {
                    trace: Some("ADC $00".to_owned()),
                    cpu_cycles: 4,
                },
                json!({ "kind": "cpu_step", "trace": "ADC $00", "cpu_cycles": 4 }),
            ),
            (
                DispatchOutput::CycleCount { cpu_cycles: 100 },
                json!({ "kind": "cycle_count", "cpu_cycles": 100 }),
            ),
            (
                DispatchOutput::ControllerState {
                    controller_bits: 255,
                },
                json!({ "kind": "controller_state", "controller_bits": 255 }),
            ),
            (
                DispatchOutput::EmulatorState {
                    paused: true,
                    speed_permille: 1000,
                    controller_bits: 0,
                },
                json!({
                    "kind": "emulator_state",
                    "paused": true,
                    "speed_permille": 1000,
                    "controller_bits": 0
                }),
            ),
            (
                DispatchOutput::Registers {
                    pc: 0x8000,
                    a: 1,
                    x: 2,
                    y: 3,
                    sp: 0xFF,
                    status: 0x24,
                },
                json!({
                    "kind": "registers",
                    "pc": 0x8000,
                    "a": 1,
                    "x": 2,
                    "y": 3,
                    "sp": 0xFF,
                    "status": 0x24
                }),
            ),
            (
                DispatchOutput::Memory {
                    address: 0x2000,
                    value: 0xAA,
                },
                json!({ "kind": "memory", "address": 0x2000, "value": 0xAA }),
            ),
            (
                DispatchOutput::Fps { fps_milli: 60000 },
                json!({ "kind": "fps", "fps_milli": 60000 }),
            ),
            (
                DispatchOutput::PpuFrameCounter {
                    frame_counter: 1234,
                },
                json!({ "kind": "ppu_frame_counter", "frame_counter": 1234 }),
            ),
            (
                DispatchOutput::Frame { seq: 1, bytes: 8 },
                json!({ "kind": "frame", "seq": 1, "bytes": 8 }),
            ),
            (
                DispatchOutput::FrameCaptured {
                    path: "frame.png".to_owned(),
                    bytes: 3,
                },
                json!({ "kind": "frame_captured", "path": "frame.png", "bytes": 3 }),
            ),
            (
                DispatchOutput::Audio { seq: 5, samples: 2 },
                json!({ "kind": "audio", "seq": 5, "samples": 2 }),
            ),
            (
                DispatchOutput::StateSlot {
                    slot: "quicksave".to_owned(),
                },
                json!({ "kind": "state_slot", "slot": "quicksave" }),
            ),
            (
                DispatchOutput::RomLoaded {
                    mapper_id: 4,
                    prg_rom_bytes: 262144,
                    reset_pc: 0x8000,
                },
                json!({
                    "kind": "rom_loaded",
                    "mapper_id": 4,
                    "prg_rom_bytes": 262144,
                    "reset_pc": 0x8000
                }),
            ),
            (
                DispatchOutput::MacroExecuted {
                    frames_elapsed: 10,
                    final_controller_bits: 8,
                },
                json!({
                    "kind": "macro_executed",
                    "frames_elapsed": 10,
                    "final_controller_bits": 8,
                }),
            ),
            (
                DispatchOutput::PpuOam {
                    oam_bytes: vec![255, 128],
                },
                json!({ "kind": "ppu_oam", "oam_bytes": [255, 128] }),
            ),
            (
                DispatchOutput::DslAssembled {
                    bytes_written: 100,
                    label_count: 5,
                    nmi_vector: 0xFFFA,
                    reset_vector: 0xFFFC,
                    irq_vector: 0xFFFE,
                },
                json!({
                    "kind": "dsl_assembled",
                    "bytes_written": 100,
                    "label_count": 5,
                    "nmi_vector": 0xFFFA,
                    "reset_vector": 0xFFFC,
                    "irq_vector": 0xFFFE,
                }),
            ),
            (
                DispatchOutput::DslRomLoaded {
                    mapper_id: 0,
                    prg_rom_bytes: 16384,
                    reset_pc: 0xC000,
                    rom_bytes: 2,
                },
                json!({
                    "kind": "dsl_rom_loaded",
                    "mapper_id": 0,
                    "prg_rom_bytes": 16384,
                    "reset_pc": 0xC000,
                    "rom_bytes": 2,
                }),
            ),
            (
                DispatchOutput::DslRomExported {
                    path: "rom.nes".to_owned(),
                    bytes: 3,
                    mapper_id: 0,
                    prg_rom_bytes: 32768,
                },
                json!({
                    "kind": "dsl_rom_exported",
                    "path": "rom.nes",
                    "bytes": 3,
                    "mapper_id": 0,
                    "prg_rom_bytes": 32768,
                }),
            ),
            (
                DispatchOutput::DslRomExportedBase64 {
                    rom_base64: "TkVT".to_owned(),
                    bytes: 3,
                    mapper_id: 0,
                    prg_rom_bytes: 32768,
                },
                json!({
                    "kind": "dsl_rom_exported_base64",
                    "rom_base64": "TkVT",
                    "bytes": 3,
                    "mapper_id": 0,
                    "prg_rom_bytes": 32768,
                }),
            ),
        ];

        for (output, expected) in cases {
            assert_eq!(dispatch_output_value(output), expected);
        }
    }
}
