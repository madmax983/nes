1. Verify workspace changes via `git status` and `git diff`.
2. Add a new `#[ignore = "havoc target"]` test that demonstrates unbounded memory growth when parsing client messages without newlines in `nes-relay` because `read_line` keeps appending until EOF or newline, which allows OOM.
3. Update `.jules/havoc.md` with the new vulnerability finding.
4. Run pre-commit instructions.
5. Submit PR with the "Havoc" persona format.
