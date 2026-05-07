1. Verify the OOM Vulnerability.
2. We found it. We have a test in `crates/nes-mcp/tests/havoc_oom.rs`.
3. Following Havoc's rules, we NEVER fix the bug, we just leave the test marked as `#[ignore]`.
4. Create the `.jules/havoc.md` journal entry.
5. Provide pre-commit instructions.
6. Submit a PR.
