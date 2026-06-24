use nes_mcp::DispatchOutput;
use nes_mcp::protocol::{
    RpcError, dispatch_output_value, json_arg_to_string, jsonrpc_error, jsonrpc_result,
    map_tool_arguments,
};
use serde_json::Value;
use serde_json::json;

#[test]
fn test_rpc_error_codes() {
    let parse = RpcError::parse_error("test");
    assert_eq!(parse.code, -32700);
    assert_eq!(parse.message, "test");

    let invalid_req = RpcError::invalid_request("test");
    assert_eq!(invalid_req.code, -32600);
    assert_eq!(invalid_req.message, "test");

    let method_not_found = RpcError::method_not_found("test");
    assert_eq!(method_not_found.code, -32601);
    assert_eq!(method_not_found.message, "test");

    let invalid_params = RpcError::invalid_params("test");
    assert_eq!(invalid_params.code, -32602);
    assert_eq!(invalid_params.message, "test");

    let internal_error = RpcError::internal_error("test");
    assert_eq!(internal_error.code, -32603);
    assert_eq!(internal_error.message, "test");

    // Also test into_json logic
    assert!(parse.into_json().is_object());

    let mut err_with_data = RpcError::internal_error("test");
    err_with_data.data = Some(json!({"details": "none"}));
    let err_json = err_with_data.into_json();
    assert_eq!(err_json["data"]["details"], "none");
}

#[test]
fn test_map_tool_arguments() {
    let mut args = serde_json::Map::new();
    args.insert("foo".to_owned(), json!("bar"));
    let map = map_tool_arguments(args);
    assert_eq!(map.get("foo").unwrap(), "bar");
}

#[test]
fn test_json_arg_to_string() {
    let hello = String::from("hello");
    let value = Value::String(hello.clone());
    let res = json_arg_to_string(value);
    assert_eq!(res, "hello");

    assert_eq!(json_arg_to_string(json!(42)), "42");
    assert_eq!(json_arg_to_string(json!(true)), "true");
}

#[test]
fn test_jsonrpc_result() {
    let result = jsonrpc_result(Value::Number(42.into()), Value::String("success".into()));
    assert_eq!(result["jsonrpc"], "2.0");
    assert_eq!(result["id"], 42);
    assert_eq!(result["result"], "success");
}

#[test]
fn test_jsonrpc_error() {
    let err = RpcError::invalid_request("bad req");
    let result = jsonrpc_error(Value::Number(43.into()), err);
    assert_eq!(result["jsonrpc"], "2.0");
    assert_eq!(result["id"], 43);
    assert_eq!(result["error"]["code"], -32600);
    assert_eq!(result["error"]["message"], "bad req");
}

#[test]
fn test_dispatch_output_value() {
    let output = DispatchOutput::Ack;
    let value = dispatch_output_value(output);
    assert_eq!(value["kind"], "ack");
}

#[test]
fn test_tool_input_schema() {
    use nes_mcp::protocol::tool_input_schema;

    let schema = tool_input_schema("set_controller_state");
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["bits"].is_object());

    let schema2 = tool_input_schema("press_button");
    assert_eq!(schema2["type"], "object");
    assert!(schema2["properties"]["button"].is_object());

    let schema3 = tool_input_schema("set_speed");
    assert_eq!(schema3["type"], "object");
    assert!(schema3["properties"]["multiplier"].is_object());

    let schema4 = tool_input_schema("read_memory");
    assert_eq!(schema4["type"], "object");
    assert!(schema4["properties"]["address"].is_object());

    let schema5 = tool_input_schema("get_frame");
    assert_eq!(schema5["type"], "object");

    let schema6 = tool_input_schema("capture_frame");
    assert_eq!(schema6["type"], "object");
    assert!(schema6["properties"]["path"].is_object());

    let schema7 = tool_input_schema("save_state");
    assert_eq!(schema7["type"], "object");
    assert!(schema7["properties"]["slot"].is_object());

    let schema8 = tool_input_schema("load_rom");
    assert_eq!(schema8["type"], "object");
    assert!(schema8["properties"]["rom_path"].is_object());
    assert!(schema8["properties"]["rom_hex"].is_object());

    let schema9 = tool_input_schema("assemble_6502_dsl");
    assert_eq!(schema9["type"], "object");
    assert!(schema9["properties"]["source"].is_object());

    let schema10 = tool_input_schema("load_6502_dsl");
    assert_eq!(schema10["type"], "object");
    assert!(schema10["properties"]["source"].is_object());

    let schema11 = tool_input_schema("export_6502_dsl_rom");
    assert_eq!(schema11["type"], "object");
    assert!(schema11["properties"]["source"].is_object());

    let schema12 = tool_input_schema("export_6502_dsl_rom_base64");
    assert_eq!(schema12["type"], "object");
    assert!(schema12["properties"]["source"].is_object());

    let schema_unmatched = tool_input_schema("unknown");
    assert_eq!(
        schema_unmatched,
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    );
}
