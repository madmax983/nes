1. **Flatten `classify_keyboard_input` in `crates/nes-desktop/src/input.rs`**
   - The current implementation uses `if let Some(...) else { return ... }` but we can use early return guard clauses to prevent deep nesting. Wait, `classify_keyboard_input` isn't nested deeply anymore. Let's look at `execute_app_action` in `crates/nes-desktop/src/main.rs`.

2. **Extract `handle_keyboard_input` from the massive window event loop in `crates/nes-desktop/src/main.rs`**
   - In `main.rs`, the `KeyboardInput` arm inside `match classify_window_event(&event)` is around 115 lines long. This is a classic "God Function" smell because the keyboard handling logic is mixed inside the event loop closure.
   - We will extract this logic into a dedicated helper function `handle_keyboard_input(key, pressed, ctx, control_flow)`. We will pass `AppContext` to avoid passing 15 different mutable variables. We will use a python script or `replace_with_git_merge_diff` to extract it. Wait, `AppContext` contains `keyboard_bits`, but `keyboard_bits` is borrowed mutably inside the loop while other things are borrowed mutably. Let's check `build_ctx!()`.

3. **Refactor duplicate calls to `set_overlay_open` in `execute_app_action`**
   - In `crates/nes-desktop/src/main.rs`, `AppAction::ToggleOverlay`, `AppAction::Resume`, `AppAction::OpenRom`, `AppAction::Reset`, and `AppAction::Quit` all call `set_overlay_open(...)` multiple times with the exact same long list of arguments (`ctx.overlay, ctx.core, ctx.audio_output, ctx.window, ctx.session`).
   - We will define a helper closure inside `execute_app_action`:
     ```rust
     let mut set_overlay = |open: bool, ctx: &mut AppContext<'_>| {
         set_overlay_open(ctx.overlay, open, ctx.core, ctx.audio_output, ctx.window, ctx.session)
     };
     ```
   - We will replace the multiple redundant explicit calls with `set_overlay(open, ctx)?`. This directly reduces visual noise and boilerplate.

4. **Verify the refactor.**
   - Run `cargo fmt`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-targets --all-features` in bash to verify that no logic was broken.

5. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.**

6. **Submit PR.**
   - Use the `submit` tool with title `⚒️ Forge: Refactor redundant set_overlay_open calls` and a properly formatted description.
