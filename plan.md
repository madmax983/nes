1. **Analyze Coverage Failure:**
   - The CI is failing because Codecov shows a drop in code coverage ("31.81% of diff hit").
   - This happens when extracting code that is untested or only partially tested into new functions. The new functions `resolve_netplay_config` and `resolve_rta_config` are marked as untested or partially tested because not all logical branches (like early returns or `unwrap_or_else` paths) are hit during the standard test suite.
2. **Add Unit Tests:**
   - Write unit tests in `crates/nes-desktop/src/config.rs` to cover `resolve_netplay_config` and `resolve_rta_config`.
   - The tests need to verify:
     - `resolve_netplay_config` returns `None` when `netplay_enabled` is false.
     - `resolve_netplay_config` returns a populated config when enabled.
     - `resolve_rta_config` returns `None` when disabled.
     - `resolve_rta_config` returns a populated config when enabled.
3. **Verify Tests:**
   - Run `cargo test --all-features` in `nes-desktop`.
   - Run `cargo llvm-cov` if available to check local coverage (or just rely on the tests proving functionality).
4. **Pre-commit:**
   - Complete pre-commit steps.
5. **Submit Fix:**
   - Call the `submit` tool.
