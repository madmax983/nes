with open("crates/nes-desktop/src/main.rs", "r") as f:
    content = f.read()

import re

macro_start = "    macro_rules! build_ctx {"
macro_regex = re.compile(r"    macro_rules! build_ctx \{.*?\n    \}", re.DOTALL)

flat_macro = "    macro_rules! build_ctx { () => { AppContext { core: &mut core, session: &mut session, session_cheats: &mut session_cheats, overlay: &mut overlay, rollback_enabled: rollback.is_some(), runtime: &runtime, audio_output: audio_output.as_ref(), time_machine: &mut time_machine, rewind_held: &mut rewind_held, metrics: &mut metrics, keyboard_bits, gamepad_bits: &mut gamepad_bits, window: &window, rta_manager: &mut rta_manager, frame_index } }; }"

new_content = macro_regex.sub(flat_macro, content)

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(new_content)
