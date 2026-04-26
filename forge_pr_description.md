🗑️ Smell
The codebase contained instances of `unwrap_or(Value::Null)` when extracting the request ID in `crates/nes-mcp/src/main.rs`. Providing an explicit `Value::Null` violates clippy's `unwrap_or_default` lint and goes against idiomatic Rust practices, since `Value::Null` is the implementation of `Default` for `serde_json::Value`.

✨ Solution
Replaced `unwrap_or(Value::Null)` with `unwrap_or_default()` in `crates/nes-mcp/src/main.rs`. This cleans up the code and aligns it with standard Rust idioms.

🧼 Benefit
Reduces cognitive load and makes the code slightly more concise. Adheres strictly to clippy lints, preventing future warnings and standardizing the codebase.

🛡️ Verification
Ran `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test`, and `cargo fmt --all`. Tests passed. No logic changed.
