# Havoc Journal

## 2024-04-09 - Mutex Poisoning in MCP
🧨 **The Trigger:** A thread panics while holding the global `OutputState` Mutex in `nes-mcp`.
📉 **The Stack Trace:** Subsequent calls to `frame_chunk` crash with `expect("output state lock")` on a poisoned lock.
🧪 **Reproduction:** `cargo test --test havoc_output_dos -- --ignored`
😈 **Comment:** You assumed the user closure would never panic. You were wrong.

## 2025-04-07 - MCP Daemon OOM Crash
🧨 **The Trigger:** A malicious payload with an impossibly large `Content-Length`.
📉 **The Stack Trace:** No explicit stack trace outputted; the test `havoc_crash_mcp_daemon_oom` passed, meaning the daemon crashed as expected from the OOM attack.
🧪 **Reproduction:** Run `cargo test -p nes-mcp --test mcp_havoc_crash`
😈 **Comment:** You assumed the client would not send 18 Exabytes. You were wrong.

## 2025-04-07 - Macro Engine DoS Hang
🧨 **The Trigger:** A WAIT command with u64::MAX frames in the MCP macro engine.
📉 **The Stack Trace:** (Thread hangs and eventually panics due to test timeout)
```
thread 'havoc_crash_mcp_dos_wait_frames' panicked at crates/nes-mcp/tests/havoc.rs:40:10:
timeout
```
🧪 **Reproduction:** Run `cargo test -p nes-mcp --test havoc havoc_crash_mcp_dos_wait_frames`
😈 **Comment:** You assumed nobody would want to WAIT longer than the universe has existed. You were wrong.
## 2024-04-09 - Concurrency Deadlock in Client Cleanup
🧨 **The Trigger:** Loom discovers a deadlock / race condition when cleaning up a relay client from the global room map concurrently.
📉 **The Stack Trace:** Loom model trace panics on deadlock detection.
🧪 **Reproduction:** `RUSTFLAGS="--cfg loom" cargo test --test havoc_loom_deadlock -- --ignored`
😈 **Comment:** "Thread-safe" is a lie until proven by `loom`.
