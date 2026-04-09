# Havoc Journal

## 2024-04-09 - Mutex Poisoning in MCP
🧨 **The Trigger:** A thread panics while holding the global `OutputState` Mutex in `nes-mcp`.
📉 **The Stack Trace:** Subsequent calls to `frame_chunk` crash with `expect("output state lock")` on a poisoned lock.
🧪 **Reproduction:** `cargo test --test havoc_output_dos -- --ignored`
😈 **Comment:** You assumed the user closure would never panic. You were wrong.

## 2024-04-09 - Concurrency Deadlock in Client Cleanup
🧨 **The Trigger:** Loom discovers a deadlock / race condition when cleaning up a relay client from the global room map concurrently.
📉 **The Stack Trace:** Loom model trace panics on deadlock detection.
🧪 **Reproduction:** `RUSTFLAGS="--cfg loom" cargo test --test havoc_loom_deadlock -- --ignored`
😈 **Comment:** "Thread-safe" is a lie until proven by `loom`.
