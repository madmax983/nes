1. **Verify Existing Workspace Changes:**
   - Execute `git status` and `git diff --staged` using `run_in_bash_session` to confirm existing changes (such as the addition of `crates/nes-mcp/tests/havoc_mcp_output_race.rs`).
2. **Log Findings to Havoc Journal:**
   - Use `run_in_bash_session` to append a wreckage log to `.jules/havoc.md` using the following exact Heredoc script:
     ```bash
     cat << 'INNER_EOF' >> .jules/havoc.md

     ## YYYY-MM-DD - Output State Lock Contention
     🧨 **The Trigger:** Concurrent publishing of frames/audio to the global output state.
     📉 **The Stack Trace:** (Simulated via loom race failure in tests)
     🧪 **Reproduction:** Run `cargo test -p nes-mcp --test havoc_mcp_output_race`
     😈 **Comment:** Global Mutex locks on hot-paths with multiple producers are a recipe for thread starvation and deadlocks. You were warned.
     INNER_EOF
     ```
3. **Verify Havoc Journal Update:**
   - Use `run_in_bash_session` to execute `cat .jules/havoc.md` to confirm the log was appended successfully.
4. **Execute Pre-commit Verification:**
   - Use `run_in_bash_session` to execute:
     - `cargo clippy --all-targets --all-features -- -D warnings`
     - `cargo test`
     - `cargo fmt --all`
   - Ensure these commands pass cleanly before proceeding to submission.
5. **Pre-commit Checks:**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
6. **Submit the PR:**
   - Verify the current active branch using `git status` via `run_in_bash_session`.
   - Call the `submit` tool to create a PR on the active branch.
   - The PR title will be precisely `👺 Havoc: Output State Lock Contention`.
   - The PR description will include the following sections exactly:
     - `🧨 **The Trigger:** "Concurrent publishing of frames/audio to the global output state."`
     - `📉 **The Stack Trace:** "(Simulated via loom race failure in tests)"`
     - `🧪 **Reproduction:** "Run \`cargo test -p nes-mcp --test havoc_mcp_output_race\`."`
     - `😈 **Comment:** "Global Mutex locks on hot-paths with multiple producers are a recipe for thread starvation and deadlocks. You were warned."`
