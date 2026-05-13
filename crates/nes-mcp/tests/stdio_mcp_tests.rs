use serde_json::{Value, json};
use std::io::{Read, Write};
use std::process::{Command, Stdio};

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
        let header_offset = match raw[cursor..]
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
        {
            Some(o) => o,
            None => break,
        };
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
        if payload_end > raw.len() {
            break;
        }
        let payload = &raw[cursor..payload_end];
        responses.push(serde_json::from_slice(payload).expect("response payload parses"));
        cursor = payload_end;
    }

    responses
}

#[test]
fn test_stdio_mcp_invalid_arguments_type() {
    let mut mcpd = Command::new(env!("CARGO_BIN_EXE_nes-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn mcp daemon");

    let mut stdin = mcpd.stdin.take().expect("mcp daemon stdin");
    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "load_rom",
            "arguments": "this is not an object"
        }
    });
    stdin
        .write_all(&frame_message(&req))
        .expect("write request");
    drop(stdin);

    let mut stdout = mcpd.stdout.take().expect("mcp daemon stdout");
    let mut out_bytes = Vec::new();
    stdout.read_to_end(&mut out_bytes).expect("read stdout");

    mcpd.wait().expect("wait for mcp daemon");

    let responses = parse_responses(&out_bytes);
    assert_eq!(responses.len(), 1);

    let resp = &responses[0];
    if resp.get("error").is_none() {
        let result = resp.get("result").expect("valid test data");
        let is_error = result
            .get("isError")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        assert!(is_error, "Should be an error");
    }
}

#[test]
fn test_stdio_mcp_valid_arguments() {
    let mut mcpd = Command::new(env!("CARGO_BIN_EXE_nes-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn mcp daemon");

    let mut stdin = mcpd.stdin.take().expect("mcp daemon stdin");
    let req = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": "read_memory",
            "arguments": {
                "address": "0x0000",
                "length": "10"
            }
        }
    });
    stdin
        .write_all(&frame_message(&req))
        .expect("write request");
    drop(stdin);

    let mut stdout = mcpd.stdout.take().expect("mcp daemon stdout");
    let mut out_bytes = Vec::new();
    stdout.read_to_end(&mut out_bytes).expect("read stdout");

    mcpd.wait().expect("wait for mcp daemon");

    let responses = parse_responses(&out_bytes);
    assert_eq!(responses.len(), 1);

    let resp = &responses[0];
    let result = resp.get("result").expect("valid test data");
    let is_error = result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    assert!(!is_error, "Should not be an error");

    let content = result
        .get("content")
        .expect("valid test data")
        .as_array()
        .expect("valid test data");
    let text = content[0]
        .get("text")
        .expect("valid test data")
        .as_str()
        .expect("valid test data");
    assert!(text.contains("ok"), "Output should contain ok");
}

#[test]
fn test_stdio_mcp_payload_size_limit() {
    // 10 * 1024 * 1024 = 10485760
    let exact_limit = 10 * 1024 * 1024;
    let tests = [
        exact_limit,
        exact_limit + 1,
        exact_limit - 1,
        10 + 1024 + 1024,
        10 * 1024 + 1024,
    ];

    for len in tests {
        let mut mcpd = Command::new(env!("CARGO_BIN_EXE_nes-mcp"))
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn mcp daemon");

        let mut stdin = mcpd.stdin.take().expect("mcp daemon stdin");
        let bad_req = format!("Content-Length: {}\r\n\r\n", len).into_bytes();
        stdin.write_all(&bad_req).expect("write request");
        // Don't send the body, just let it fail on limit or block
        // Wait, if we send exact_limit, it will block waiting for body, so we need to kill it
        // Or better, send EOF immediately, which will cause it to fail reading payload body
        drop(stdin);

        let mut stderr = mcpd.stderr.take().expect("mcp daemon stderr");
        let mut err_bytes = Vec::new();
        stderr.read_to_end(&mut err_bytes).expect("read stderr");
        let err_str = String::from_utf8_lossy(&err_bytes);

        mcpd.wait().expect("wait for mcp daemon");

        if len > exact_limit {
            assert!(
                err_str.contains("exceeds maximum allowed size"),
                "Expected size limit error for len {}",
                len
            );
        } else {
            // Should fail with failed reading payload body because we sent EOF
            assert!(
                err_str.contains("failed reading payload body"),
                "Expected read error for len {}",
                len
            );
        }
    }
}
