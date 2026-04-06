1. **Target:** `crates/nes-relay/src/main.rs`
2. **Action:** Refactor `read_client_message` to bound the payload size. By using `reader.take(MAX_MESSAGE_BYTES)`, we prevent malicious clients from sending unbounded streams of bytes without newlines, causing memory exhaustion (OOM). We'll also use `io::Read` to bring `take()` into scope.
3. **Verification:** I will write a test that verifies `read_client_message` correctly drops lines larger than `MAX_MESSAGE_BYTES`.
