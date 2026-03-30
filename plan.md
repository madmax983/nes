1. **Refactor nested match block in `crates/nes-core/src/api.rs`:**
   - Modify the `apply_mapper_delta` function which contains a deeply nested match block over `delta.kind` extracting inner mappers via `if let`. I'll rewrite this to pattern match both values `(&delta.kind, &mut *self)` directly using `match`, eliminating nested blocks and reducing boilerplate code drastically.
2. **Refactor empty match arms in `crates/nes-core/src/api.rs`:**
   - Simplify a match statement over `resolved` inside `read` that uses a `_ => {}` empty match arm, turning it into a concise `if matches!(resolved, ...)` statement.
3. **Refactor `build_cnrom` and `build_nrom` in `crates/nes-core/src/api.rs`:**
   - The current code uses empty match arms `PRG_16K_BYTES | PRG_32K_BYTES => {}` or similar to do validation, which can be cleanly inverted with an `if !matches!(...)` guard clause.
4. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**
5. **Submit the change.**
   - Once all tests pass, I will submit the PR with a description explaining the readability improvements in accordance with Forge's philosophy.
