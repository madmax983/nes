1. **Identify Performance Optimization**
   - After analyzing the codebase for potential optimizations that adhere to Bolt's boundaries, the `split_csv` function in `nes-dsl` creates a new vector via `Vec::new()` to hold comma-separated items on every call. This occurs twice: once in `nes-dsl/src/lib.rs` and once in `nes-dsl/src/parser.rs`.
   - By replacing `Vec::new()` with `Vec::with_capacity(4)` in both locations, we can eliminate a heap allocation resize for the common case where comma-separated lists contain multiple small items.
   - Also changed `Vec::new()` to `Vec::with_capacity(32)` in the `Assembler::new()` function for the `fixups` vector, which frequently accumulates items during assembly.

2. **Implement the optimization**
   - (Done) I've successfully replaced `Vec::new()` with `Vec::with_capacity(...)` in `crates/nes-dsl/src/lib.rs`, `crates/nes-dsl/src/parser.rs`, and `crates/nes-dsl/src/assembler.rs`.
   - Run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, and `cargo fmt --all`.
   - Update `.jules/bolt.md` with the critical learning if applicable.

3. **Complete pre-commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

4. **Submit the PR**
   - Title: `⚡ Bolt: [performance improvement]`
   - Description includes What, Why, Impact, and Measurement.
