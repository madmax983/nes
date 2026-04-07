# Havoc Journal

## 2025-04-07 - Mutex Poisoning in MCP output_state
🧨 **The Trigger:** A panic within the `publish_frame_with` closure while holding the global `output_state` lock.
📉 **The Stack Trace:** No explicit stack trace outputted; the test `havoc_test_poisoned_mutex_on_panic` passed, confirming that subsequent accesses panic with "output state lock: PoisonError" because the global Mutex is poisoned.
🧪 **Reproduction:** Run `cargo test -p nes-mcp --test havoc_output`
😈 **Comment:** You assumed closures passed to `publish_frame_with` would never panic. You were wrong.

## 2025-04-07 - MCP Daemon OOM Crash
🧨 **The Trigger:** A malicious payload with an impossibly large `Content-Length`.
📉 **The Stack Trace:** No explicit stack trace outputted; the test `havoc_crash_mcp_daemon_oom` passed, meaning the daemon crashed as expected from the OOM attack.
🧪 **Reproduction:** Run `cargo test -p nes-mcp --test mcp_havoc_crash`
😈 **Comment:** You assumed the client would not send 18 Exabytes. You were wrong.
