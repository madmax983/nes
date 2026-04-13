1. **Update Havoc Journal**:
   - Run `run_in_bash_session` to append the destructive learning to `.jules/havoc.md` using the exact format:
```markdown

**Havoc OOM Attack on MCP Host**
**The Trigger:** Malicious client sends `Content-Length: 18446744073709551615` to MCP host, causing an immediate OOM panic due to unvalidated `vec![0_u8; len]` allocation.
**The Stack Trace:** `thread 'havoc_mcp_read_framed_dos' panicked at alloc/src/raw_vec.rs: capacity overflow`
**Reproduction:** Run `cargo test -p nes-desktop --test havoc_mcp_dos -- --ignored`
**Comment:** You assumed users wouldn't send arbitrary headers. You were wrong.
```

2. **Run Test Suite**:
   - Run `run_in_bash_session` with `cargo test --all-targets --all-features` to ensure the new ignored test does not break standard CI pipelines.

3. **Pre-commit Steps**:
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

4. **Submit Change**:
   - Run `submit` to commit the test harness and journal update with the branch name `havoc-mcp-dos-oom`, title `👺 Havoc: MCP Host OOM via Unbounded Content-Length Allocation`, and the exact description:
```markdown
🧨 **The Trigger:** Malicious client sends `Content-Length: 18446744073709551615` to MCP host, causing an immediate OOM panic due to unvalidated `vec![0_u8; len]` allocation in `read_framed_message`.
📉 **The Stack Trace:** `thread 'havoc_mcp_read_framed_dos' panicked at alloc/src/raw_vec.rs: capacity overflow`
🧪 **Reproduction:** Run `cargo test -p nes-desktop --test havoc_mcp_dos -- --ignored`
😈 **Comment:** You assumed users wouldn't send arbitrary headers. You were wrong.
```
