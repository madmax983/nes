1. Refactor `classify_keyboard_input` in `crates/nes-desktop/src/input.rs`
   - Use `match key` with guard clauses (`if pressed` etc.) to replace the sequence of `if`/`return` statements.
   - This flattens the "Pyramid of Doom" and makes the mapping of keys to `KeyboardDecision` much clearer and more idiomatic in Rust.

2. Verify Changes
   - Run `cargo fmt --all`.
   - Run `cargo clippy --all-targets --all-features -- -D warnings`.
   - Run `cargo test --all-targets --all-features`.

3. Update Journal
   - Append to `.jules/forge.md`:
     `**[Refactoring classify_keyboard_input]
**Learning:** Found cascading `if` statements with early returns in `classify_keyboard_input` which obscured the simple key-to-action mapping.
**Action:** Replaced the cascading `if` statements with a single `match` expression using guard clauses to flatten the logic and improve readability.`

4. Complete pre-commit steps
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

5. Submit PR
   - Use the `submit` tool to create a PR.
   - Title: "⚒️ Forge: Flatten classify_keyboard_input"
   - Description sections: `🚮 Smell`, `✨ Solution`, `🧼 Benefit`, `🛡️ Verification`.
