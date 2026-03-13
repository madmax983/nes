import re

with open("crates/nes-desktop/src/main.rs", "r") as f:
    content = f.read()

struct_def = """
struct AppActionContext<'a> {
    core: &'a mut NesCore,
    session: &'a mut LoadedRomSession,
    session_cheats: &'a mut SessionCheats,
    overlay: &'a mut OverlayModel,
    rollback_enabled: bool,
    runtime: &'a RuntimeConfig,
    audio_output: Option<&'a AudioOutput>,
    time_machine: &'a mut TimeMachine,
    rewind_held: &'a mut bool,
    metrics: &'a mut PerfMetrics,
    keyboard_bits: u8,
    gamepad_bits: &'a mut [u8; 2],
    window: &'a Window,
    rta_manager: &'a mut Option<RtaManager>,
    frame_index: u64,
}

"""

# Replace the fn definition
old_fn = """fn execute_app_action(
    action: AppAction,
    core: &mut NesCore,
    session: &mut LoadedRomSession,
    session_cheats: &mut SessionCheats,
    overlay: &mut OverlayModel,
    rollback_enabled: bool,
    runtime: &RuntimeConfig,
    audio_output: Option<&AudioOutput>,
    time_machine: &mut TimeMachine,
    rewind_held: &mut bool,
    metrics: &mut PerfMetrics,
    keyboard_bits: u8,
    gamepad_bits: &mut [u8; 2],
    window: &Window,
    rta_manager: &mut Option<RtaManager>,
    frame_index: u64,
) -> Result<bool, String> {"""

new_fn = """fn execute_app_action(
    action: AppAction,
    ctx: AppActionContext<'_>,
) -> Result<bool, String> {"""

content = content.replace(old_fn, struct_def + new_fn)

# Replace the variables inside the function body
vars_to_replace = ["core", "session", "session_cheats", "overlay", "rollback_enabled", "runtime", "audio_output", "time_machine", "rewind_held", "metrics", "keyboard_bits", "gamepad_bits", "window", "rta_manager", "frame_index"]

# Extract the function body
fn_start = content.find(new_fn)
fn_body_start = fn_start + len(new_fn)

# find end of fn body by counting braces
brace_count = 1
idx = fn_body_start
while brace_count > 0 and idx < len(content):
    if content[idx] == '{':
        brace_count += 1
    elif content[idx] == '}':
        brace_count -= 1
    idx += 1
fn_body_end = idx

body = content[fn_body_start:fn_body_end]

for v in vars_to_replace:
    # Use regex to only replace whole words, not if part of another word or if it already has ctx.
    body = re.sub(r'\b(?<!ctx\.)' + v + r'\b', f'ctx.{v}', body)

content = content[:fn_body_start] + body + content[fn_body_end:]

# Replace the call site
old_call = """        match execute_app_action(
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

new_call = """        match execute_app_action(
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

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(content)
