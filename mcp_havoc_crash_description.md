🧨 **The Trigger:**
- `havoc_closure_deadlock.rs`: Calling `frame_chunk` (which acquires the lock) INSIDE the closure of `publish_frame_with` (which already holds the lock).
- `havoc.rs`: A `WAIT` command with an enormous `u64` (`18446744073709551610`) will hang the thread forever.
- `mcp_havoc_crash.rs`: The payload tells the MCP daemon to allocate 18.44 Exabytes of memory (`Content-Length: 18446744073709551615`) for a JSON-RPC request.

📉 **The Stack Trace:**
- Tests purposefully hang or trigger OOM panics via SIGKILL. The deadlock test hangs inside `std::sync::Mutex::lock`.

🧪 **Reproduction:**
Run the following commands:
- `cargo test --test havoc_closure_deadlock`
- `cargo test --test havoc havoc_test_wait_macro_dos`
- `cargo test --test mcp_havoc_crash`

😈 **Comment:**
"You assumed the buffer would never be larger than RAM, and that users would never write macros that wait forever. You were wrong."
