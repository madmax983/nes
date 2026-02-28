use std::io::{Read, Write};
use std::process::{Command, Stdio};

use serde_json::{Value, json};

fn frame_message(request: &Value) -> Vec<u8> {
    let body = serde_json::to_vec(request).expect("request serialization");
    let mut framed = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
    framed.extend_from_slice(&body);
    framed
}

fn parse_responses(raw: &[u8]) -> Vec<Value> {
    let mut cursor = 0usize;
    let mut responses = Vec::new();

    while cursor < raw.len() {
        let header_offset = raw[cursor..]
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("response header delimiter");
        let header_end = cursor + header_offset;
        let headers = std::str::from_utf8(&raw[cursor..header_end]).expect("headers are utf-8");
        let content_length = headers
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length:"))
            .map(str::trim)
            .expect("content-length header")
            .parse::<usize>()
            .expect("content-length parses");

        cursor = header_end + 4;
        let payload_end = cursor + content_length;
        let payload = &raw[cursor..payload_end];
        responses.push(serde_json::from_slice(payload).expect("response payload parses"));
        cursor = payload_end;
    }

    responses
}

#[test]
fn daemon_stdio_round_trip_supports_initialize_and_tools() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nes-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn nes-mcp daemon");

    let initialize = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {}
        }
    });
    let tools_list = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });
    let tool_call = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "get_emulator_state",
            "arguments": {}
        }
    });

    {
        let stdin = child.stdin.as_mut().expect("child stdin");
        stdin
            .write_all(&frame_message(&initialize))
            .expect("write initialize frame");
        stdin
            .write_all(&frame_message(&tools_list))
            .expect("write tools/list frame");
        stdin
            .write_all(&frame_message(&tool_call))
            .expect("write tools/call frame");
        stdin.flush().expect("flush stdin");
    }
    drop(child.stdin.take());

    let mut stdout = Vec::new();
    child
        .stdout
        .take()
        .expect("child stdout")
        .read_to_end(&mut stdout)
        .expect("read daemon responses");

    let status = child.wait().expect("wait on daemon");
    assert!(status.success(), "daemon exited with status: {status}");

    let responses = parse_responses(&stdout);
    assert_eq!(
        responses.len(),
        3,
        "unexpected response count: {responses:?}"
    );

    assert_eq!(responses[0]["id"], json!(1));
    assert_eq!(
        responses[0]["result"]["serverInfo"]["name"],
        json!("nes-mcpd")
    );

    assert_eq!(responses[1]["id"], json!(2));
    let tools = responses[1]["result"]["tools"]
        .as_array()
        .expect("tools/list returns array");
    assert!(
        tools.iter().any(|tool| tool["name"] == json!("load_rom")),
        "load_rom tool missing from catalog"
    );

    assert_eq!(responses[2]["id"], json!(3));
    assert_eq!(responses[2]["result"]["isError"], json!(false));
    assert_eq!(
        responses[2]["result"]["structuredContent"]["kind"],
        json!("emulator_state")
    );
}
