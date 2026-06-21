1. **Modify `set_overlay_open` signature and calls in `crates/nes-desktop/src/main.rs`:**
   Execute the following command in `run_in_bash_session`:
   ```bash
   cat << 'PYEOF' > rewrite.py
import re

with open('crates/nes-desktop/src/main.rs', 'r') as f:
    content = f.read()

# Update signature
old_sig = """fn set_overlay_open(
    overlay: &mut OverlayModel,
    open: bool,
    core: &mut NesCore,
    audio_output: Option<&AudioOutput>,
    window: &Window,
    session: &LoadedRomSession,
) -> Result<(), String> {"""

new_sig = """fn set_overlay_open(ctx: &mut AppContext<'_>, open: bool) -> Result<(), String> {"""

content = content.replace(old_sig, new_sig)

# Update the implementation inside set_overlay_open
old_impl = """    if open {
        overlay.open();
        reconcile_core_pause_with_overlay(core, true)?;
        if let Some(output) = audio_output {
            output.clear();
        }
    } else {
        overlay.close();
        reconcile_core_pause_with_overlay(core, false)?;
    }
    window.set_title(&window_title(session, overlay.is_open()));
    Ok(())"""

new_impl = """    if open {
        ctx.overlay.open();
        reconcile_core_pause_with_overlay(ctx.core, true)?;
        if let Some(output) = ctx.audio_output {
            output.clear();
        }
    } else {
        ctx.overlay.close();
        reconcile_core_pause_with_overlay(ctx.core, false)?;
    }
    ctx.window.set_title(&window_title(ctx.session, ctx.overlay.is_open()));
    Ok(())"""

content = content.replace(old_impl, new_impl)

# Update call sites
call_pattern = r"""set_overlay_open\(
\s*ctx\.overlay,
\s*(true|false|!ctx\.overlay\.is_open\(\)),
\s*ctx\.core,
\s*ctx\.audio_output,
\s*ctx\.window,
\s*ctx\.session,
\s*\)"""

content = re.sub(call_pattern, r"set_overlay_open(ctx, \1)", content)

with open('crates/nes-desktop/src/main.rs', 'w') as f:
    f.write(content)
PYEOF
   python3 rewrite.py
   rm rewrite.py
   ```

2. **Verify `set_overlay_open` refactoring:**
   Use `run_in_bash_session` to execute `git diff crates/nes-desktop/src/main.rs` to verify that the file modifications were applied correctly.

3. **Compile and test the workspace:**
   Use `run_in_bash_session` to execute `cargo fmt --all && cargo clippy --all-targets --all-features -- -D warnings && cargo test --workspace --all-features`.

Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

5. **Commit and Submit the PR:**
   Use `run_in_bash_session` to check out a new branch and commit the changes:
   ```bash
   git checkout -b forge-refactor-set-overlay-open
   git commit -am "⚒️ Forge: Extract AppContext in set_overlay_open"
   ```
   Then use the `submit` tool to create the PR with the branch name `forge-refactor-set-overlay-open`.
   - `pr_title`: `⚒️ Forge: Extract AppContext in set_overlay_open`
   - `pr_body`:
     ```
     🚮 Smell: `set_overlay_open` took 6 separate arguments, forcing every call site to manually pass 5 individual fields extracted from the `AppContext`. This created a pyramid of visual noise.
     ✨ Solution: Refactored `set_overlay_open` to directly accept `&mut AppContext` along with the `open` boolean, dramatically simplifying all call sites.
     🧼 Benefit: Reduces boilerplate, improves readability, and centralizes mutable state access during overlay transitions.
     🛡️ Verification: Tests passed. No logic changed.
     ```
