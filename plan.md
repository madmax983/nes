1. **Objective**: Prove system fragility without fixing it, by injecting garbage data leading to panic/crash (Havoc persona). Maintain the `.jules/havoc.md` journal and submit a PR with the Havoc layout.
2. **Current Finding**: In `crates/nes-desktop/src/mcp_host.rs`, there is a function `read_framed_message`. When a malicious client sends `Content-Length: 18446744073709551615`, the `read_framed_message` method attempts to allocate `vec![0_u8; len]` (which causes an Out-Of-Memory / capacity overflow panic). This causes the entire `mcp_host` background listener thread to panic.
3. I've added the test `havoc_mcp_host_oom` to `crates/nes-desktop/src/mcp_host.rs` under the `#[cfg(test)]` mod, which correctly crashes with "capacity overflow". Wait, `havoc_mcp_host_oom` is inside the `tests` block of `mcp_host.rs`.
4. I updated `.jules/havoc.md` with:
   - 🧨 **The Trigger:** Sending a massive `Content-Length` header in the JSON-RPC TCP connection causes `nes-desktop`'s `mcp_host` thread to attempt to allocate an impossibly large memory vector, leading to an OOM panic.
   - 📉 **The Stack Trace:** Panic at `library/alloc/src/raw_vec/mod.rs:28:5: capacity overflow`
   - 🧪 **Reproduction:** `cargo test -p nes-desktop -- --ignored havoc_mcp_host_oom`
   - 😈 **Comment:** You assumed the client would not try to send 18 Exabytes to the desktop's MCP host. You were wrong.
5. Create a git commit, push, and complete the plan.

Wait, `cargo test --all-targets --all-features` passes because `havoc_mcp_host_oom` is ignored by default! So the build is green.

I will request a plan review.
