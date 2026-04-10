plan = """1. *Fix CI failure.*
   - The `host_start_and_client_round_trip_cover_bind_addr_drain_and_dispatch` test failed due to a read timeout: "A connection attempt failed because the connected party did not properly respond after a period of time".
   - The test sets a read timeout of `500ms` on the stream: `stream.set_read_timeout(Some(Duration::from_millis(500)))`.
   - The `read_response_with_host_drain` polls up to 2 seconds, but if the underlying TCP read call returns an error that `is_retryable_read_error` detects, it sleeps 10ms and retries. Wait, maybe the `read_framed_message` inside `read_response_with_host_drain` is throwing a non-retryable error, or `read_framed_message` outside of `read_response_with_host_drain` (for ping or tools/list) is timing out?
   - The panic is `read tools/call response: failed reading header line: ... (os error 10060)`. This comes from `read_response_with_host_drain`.
   - Why would it time out? Because the host processes requests asynchronously, maybe 500ms is too short for CI? Or maybe `host.drain(&mut core)` needs to be called more than once or it's just slow.
   - Wait, `is_retryable_read_error` might not cover `os error 10060` (WSAETIMEDOUT on Windows).
   - Let's check `is_retryable_read_error` in `crates/nes-desktop/src/mcp_host.rs`.
"""
print(plan)
