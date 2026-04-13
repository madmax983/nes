1. **The Target:**
   - I have identified a significant vulnerability in `nes-desktop/src/mcp_host.rs` within the `read_framed_message` function.
   - The function parses the `Content-Length` header as a `usize` and immediately calls `vec![0_u8; len]` without any bounds checking or size validation.
   - A malicious client connecting to the MCP port could send a `Content-Length` of `18446744073709551615` (or a smaller but massive number), causing the emulator host to immediately crash due to an OOM (Out Of Memory) panic when trying to allocate the `Vec`.

2. **The Harness:**
   - I will create a new test file: `crates/nes-desktop/tests/havoc_mcp_dos.rs`.
   - The test will demonstrate the vulnerability by calling the exact pattern that causes the OOM panic.
   - I will mark the test with `#[test]`, `#[should_panic]`, and `#[ignore = "Havoc OOM Attack"]`.

3. **The Presentation:**
   - I will create a PR/Issue formatted exactly as required by the Havoc persona.

Let's do this!
