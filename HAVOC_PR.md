# 👺 Havoc: MCP Host OOM Crash via Unbounded Content-Length Allocation

🧨 **The Trigger:**
Sending a JSON-RPC request to the `nes-desktop` MCP Host with `Content-Length: 18446744073709551615` (or any extremely large number) causes the daemon to attempt to allocate an array equal to the requested size directly in `read_framed_message`. This leads to a capacity overflow panic and crashes the entire application.

📉 **The Stack Trace:**
```
thread 'mcp-host-...' panicked at alloc/src/raw_vec.rs:528:5:
capacity overflow
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

🧪 **Reproduction:**
Run `cargo test -p nes-desktop havoc_crash_mcp_host_oom --features "mcp-host" -- --ignored` which uses the test harness in `crates/nes-desktop/src/mcp_host.rs` to reproduce the vulnerability.

😈 **Comment:**
"You assumed the buffer would never be larger than RAM. You were wrong."
