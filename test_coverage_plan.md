1. **Analyze Coverage Gaps**
   - Wait for `cargo-llvm-cov` to finish installing.
   - Run `cargo llvm-cov -p nes-core --features nova` to confirm the exact missed lines.
2. **Improve Tests**
   - Update `crates/nes-core/src/experimental/input_visualizer.rs` to include tests covering all button branches and the invalid framebuffer size branch.
3. **Verify**
   - Re-run `cargo llvm-cov` to ensure 100% diff hit on `input_visualizer.rs`.
4. **Submit**
   - Call the `submit` tool with `branch_name` "nova-input-visualizer".
