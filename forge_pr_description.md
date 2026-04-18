🚬 Smell
The `parse_expr` and `parse_operand_syntax` functions in `nes-dsl` contained repetitive string prefix/suffix checking and unnecessarily verbose `match` blocks for simple sign inversion, creating unnecessary vertical bloat.

✨ Solution
- In `parse_operand_syntax`, grouped all indirect mode variants inside a single `strip_prefix('(')` check to eliminate redundant allocations/checks.
- In `parse_expr`, replaced a verbose `match` block returning `value` or `-value` with a simple mathematical `sign * value` operation.

🧊 Benefit
Reduces cognitive load, eliminates repetitive string processing overhead, and embraces idiomatic Rust arithmetic simplification.

🛡️ Verification
Tests passed. No logic changed. Verified via `cargo test` and `cargo clippy`.
