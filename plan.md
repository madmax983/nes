1. **Remove heap allocations in string conversions in `nes-desktop/src/metrics.rs`**
   - Change `add_row` closure to accept `&dyn std::fmt::Display` instead of `String`
   - Use `format_args!` instead of `format!` for formatted strings.
   - Pass integer references instead of `to_string()` results.
2. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
3. **Submit the PR with the title '⚡ Bolt: [performance improvement]' and structured description sections.**
