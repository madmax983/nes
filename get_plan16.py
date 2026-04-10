plan = """1. *Fix CI failure.*
   - The test failed on Windows: `os error 10060` which is `WSAETIMEDOUT`. The error message is: "A connection attempt failed because the connected party did not properly respond after a period of time, or established connection failed because connected host has failed to respond. (os error 10060)"
   - The string `temporarily unavailable`, `would block`, `timed out`, `non-blocking socket operation` does *not* match "failed to respond after a period of time" or "os error 10060".
   - I should add `"os error 10060"` or `"failed to respond"` to `is_retryable_read_error`. Actually, `"period of time"` or `"10060"` or `"timeout"` might be better. Let's just add `"10060"` and `"failed to respond"`.
   - I will use `replace_with_git_merge_diff` to update `is_retryable_read_error` in `crates/nes-desktop/src/mcp_host.rs`.
   - Wait, `timed out` doesn't match `WSAETIMEDOUT` message. I'll add `"10060"` to the list.
"""
print(plan)
