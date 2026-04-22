🚮 Smell:
- `validate_symbol` checked `.is_empty()` then manually `.unwrap()`'d the first char, an unsafe/ugly pattern.
- `mcp_host.rs` explicitly used `.unwrap_or(Value::Null)` instead of the idiomatic `.unwrap_or_default()`.

✨ Solution:
- Replaced the logic in `nes-dsl/src/parser.rs` with a single `let Some(...) = ... else { ... }` guard clause.
- Replaced `.unwrap_or(Value::Null)` with `.unwrap_or_default()` in `nes-desktop/src/mcp_host.rs`.

🧼 Benefit:
- Idiomatic, strictly typed without panicking edge-cases hidden behind `unwrap`.
- Cleaner, flatter function logic in both modules.

🛡️ Verification:
- Ran `cargo check`, `cargo fmt`, `cargo clippy`, and `cargo test`. All passed. No runtime logic was modified.
