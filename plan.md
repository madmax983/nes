1. **Add Havoc Tests for String and Slice Allocation vulnerabilities**
   - Use `run_in_bash_session` with a `cat << EOF >>` command to inject the `havoc_test_rom_hex_crash` test into `crates/nes-mcp/tests/havoc.rs` to expose the vulnerability in `parse_hex_bytes`.
2. **Verify changes**
   - Use `run_in_bash_session` with `cat crates/nes-mcp/tests/havoc.rs` to verify that the change was correctly applied to the test file.
3. **Clean up scratchpads**
   - Execute `rm plan2.md` to remove remaining temporary files.
4. **Execute workspace tests**
   - Execute `cargo test --all-features` to ensure no regressions were introduced across the workspace.
5. **Execute havoc test**
   - Execute `cargo test --all-features --test havoc -- --ignored` to verify the newly added test successfully simulates the OOM crash via panic.
6. **Complete pre commit steps**
   - Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
7. **Submit the change**
   - Use the `submit` tool to create the PR, ensuring the title is exactly `👹 Havoc: [TITLE]` and the description explicitly includes the mandatory sections: `🧨 **The Trigger:**`, `📉 **The Stack Trace:**`, `🧪 **Reproduction:**`, and `😈 **Comment:**`.
