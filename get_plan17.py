plan = """1. *Fix CI failure.*
   - The test failed on Windows: `os error 10060` which is `WSAETIMEDOUT`.
   - Update `is_retryable_read_error` in `crates/nes-desktop/src/mcp_host.rs` using `replace_with_git_merge_diff` to add `|| lowered.contains("10060")` and `|| lowered.contains("failed to respond")`.

```
<<<<<<< SEARCH
    fn is_retryable_read_error(message: &str) -> bool {
        let lowered = message.to_ascii_lowercase();
        lowered.contains("temporarily unavailable")
            || lowered.contains("would block")
            || lowered.contains("timed out")
            || lowered.contains("non-blocking socket operation")
    }
=======
    fn is_retryable_read_error(message: &str) -> bool {
        let lowered = message.to_ascii_lowercase();
        lowered.contains("temporarily unavailable")
            || lowered.contains("would block")
            || lowered.contains("timed out")
            || lowered.contains("non-blocking socket operation")
            || lowered.contains("10060") // WSAETIMEDOUT on Windows
            || lowered.contains("failed to respond")
    }
>>>>>>> REPLACE
```

2. *Verify the code builds and passes tests locally.*
   - Use `run_in_bash_session` to run `cargo test -p nes-desktop`.

3. *Pre-commit steps.*

4. *Submit changes.*
   - Run `submit` to commit the fix.
"""
print(plan)
