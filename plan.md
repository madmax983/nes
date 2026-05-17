1. Modify `execute_app_action` in `crates/nes-desktop/src/main.rs`.
   - Extract the match branches of `AppAction` into smaller helper functions (`handle_toggle_overlay`, `handle_resume`, `handle_open_cheats`, `handle_open_rom`, `handle_save_slot`, `handle_load_slot`, `handle_reset`).
   - Run `cargo clippy`, `cargo fmt`, `cargo test`.
2. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
3. Submit the change to the current branch with a PR titled `⚒️ Forge: Extract execute_app_action match arms` and a description containing `🚮 Smell`, `✨ Solution`, `🧹 Benefit`, and `🛡️ Verification`.
