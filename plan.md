1.  **Identify Weak Point**: Mutex poisoning in `nes_mcp::output::publish_frame_with` and `nes_mcp::output::publish_audio_with` leading to a Denial of Service (DoS).
2.  **Verify Mutex Poisoning Bug**: We have `crates/nes-mcp/tests/havoc_mcp_output_poison.rs` (frame poison panic) and created `crates/nes-mcp/tests/havoc_mcp_audio_poison.rs` (audio poison panic) and `crates/nes-mcp/tests/havoc_mcp_audio_dos.rs` which show the system fails to recover gracefully.
3.  **Write PR/Issue file**: I'll create `havoc-report.md` simulating the PR, documenting the trigger, stack trace, and reproduction step.
4.  **Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.**
5.  **Submit the PR**.
