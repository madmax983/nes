1. **Fix Codecov drop in `crates/nes-ai/src/env.rs`**
   - The addition of `if frame_stack_count > 0 { ... }` introduced an uncovered branch, because `frame_stack` is guaranteed to be strictly greater than `0` (validated during config loading and inside `FrameStack::new`).
   - We will remove the `if` check and just perform the final `self.frame_stack.push(frame);` unconditionally, alongside `saturating_sub(1)` for the loop.
2. **Run tests**
   - Run `cargo test -p nes-ai` to ensure tests still pass.
3. **Complete pre-commit steps.**
   - Run pre-commit instructions.
4. **Submit changes**
   - Submit the PR with the same branch name to update the existing PR.
