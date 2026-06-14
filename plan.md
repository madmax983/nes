1. **Refactor Event Loop Keyboard Decision Handling**
   - Extract the large `match classify_keyboard_input(key, pressed, mode)` block in `crates/nes-desktop/src/main.rs` into a standalone helper function `dispatch_keyboard_decision(decision, &mut ctx, control_flow)`.
   - Update `main.rs` to use `dispatch_keyboard_decision` instead of the inline `match` block.
2. Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.
3. Submit the change using `gh pr create` via `run_in_bash_session`.
