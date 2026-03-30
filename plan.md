1. **Refactor TUI Rendering `draw_frame` in `crates/nes-tui/src/main.rs`:**
   - Extract the nested closure within `terminal.draw` into separate named functions: `render_tui_hud`, `render_tui_controls`, and `render_tui_header`.
   - This removes the "Pyramid of Doom" and reduces the length of `draw_frame`, resolving `clippy::too_many_lines`.

2. **Refactor Desktop Overlay Rendering `draw_overlay` in `crates/nes-desktop/src/overlay.rs`:**
   - Extract the `MainMenu` and `Cheats` arms of the `match model.panel()` statement into `draw_main_menu_panel` and `draw_cheats_panel` helper functions.
   - This flattens the structure, reducing cognitive load and resolving `clippy::too_many_lines`.

3. **Verify and Finalize:**
   - Run `cargo fmt --all`.
   - Run `cargo clippy --all-targets --all-features -- -D warnings`.
   - Run `cargo test` to ensure zero behavior change.
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
