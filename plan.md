1. Verify existing tests for Havoc
- Use `run_in_bash_session` to execute `git status` and `git diff` to verify the modified `crates/nes-desktop/tests/mcp_host_slowloris.rs` test file.

2. Run Pre-commit Validation Commands
- Use `run_in_bash_session` to run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, and `cargo fmt --all`.

3. Pre-commit steps
- Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

4. Submit PR
- Use `run_in_bash_session` to run `git status` to determine the active branch, then use the `submit` tool to create a PR on that branch with the title '👺 Havoc: Add test for slowloris vulnerability in mcp-host'. The description must exactly include the sections: '🧨 **The Trigger:**', '📉 **The Stack Trace:**', '🧪 **Reproduction:**', and '😈 **Comment:**'.
