with open("crates/nes-desktop/src/main.rs", "r") as f:
    content = f.read()

# I need to fix `dispatch_app_action` call to `execute_app_action`

old_call = """    match execute_app_action(
        action,
        core,
        session,
        session_cheats,
        overlay,
        rollback_enabled,
        runtime,
        audio_output,
        time_machine,
        rewind_held,
        metrics,
        keyboard_bits,
        gamepad_bits,
        window,
        rta_manager,
        frame_index,
    ) {"""

new_call = """    match execute_app_action(
        action,
        AppActionContext {
            core,
            session,
            session_cheats,
            overlay,
            rollback_enabled,
            runtime,
            audio_output,
            time_machine,
            rewind_held,
            metrics,
            keyboard_bits,
            gamepad_bits,
            window,
            rta_manager,
            frame_index,
        },
    ) {"""

content = content.replace(old_call, new_call)

# also fix the blank line before AppActionContext
content = content.replace("#[allow(clippy::too_many_arguments)]\n\nstruct AppActionContext<'a> {", "struct AppActionContext<'a> {")

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(content)
