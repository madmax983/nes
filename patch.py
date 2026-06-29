import sys

with open("crates/nes-desktop/src/main.rs", "r") as f:
    content = f.read()

search = """        AppAction::Reset => {
            ctx.core
                .execute(Command::Reset)
                .map_err(|err| format!("Reset failed: {err}"))?;
            reset_ephemeral_state(ctx);"""

replace = """        AppAction::Reset => {
            ctx.core
                .execute(Command::Reset)
                .map_err(|err| format!("Reset failed: {err}"))?;
            apply_session_cheats(ctx.core, ctx.session_cheats)?;
            reset_ephemeral_state(ctx);"""

if search in content:
    content = content.replace(search, replace)
    with open("crates/nes-desktop/src/main.rs", "w") as f:
        f.write(content)
    print("Success")
else:
    print("Search string not found")
