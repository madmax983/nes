1. Add `#[ignore]` havoc tests to `crates/nes-relay/tests/havoc_loom_deadlock.rs` demonstrating deadlock vulnerability if relay client cleanup re-acquires a lock.
2. Add `#[ignore]` havoc tests to `crates/nes-mcp/tests/havoc_mcp_output_dos.rs` and `crates/nes-mcp/tests/havoc_mcp_audio_dos.rs` demonstrating global mutex poison DoS.
3. Update `.jules/havoc.md` with findings about the deadlock and mutex poisoning.
4. Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
5. Create a PR to submit the wreckage with the persona format.
