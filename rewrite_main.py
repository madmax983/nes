with open("crates/nes-desktop/src/main.rs", "r") as f:
    data = f.read()

import re

# Remove functions
funcs_to_remove = [
    "should_resume_after_rewind_hold",
    "release_all_buttons",
    "track_keyboard_bits_for_key",
    "resync_restored_inputs",
    "is_player_two_slot",
    "merge_local_input_bits",
    "update_button_bits",
    "apply_gamepad_delta_commands"
]

for func in funcs_to_remove:
    # Match the function definition and its body up to the closing brace
    # Assuming standard Rust formatting where functions end with a brace at the start of a line
    pattern = r'fn ' + func + r'\s*\([^)]*\)(?:\s*->\s*[^{]+)?\s*\{.*?\n\}\n'
    data = re.sub(pattern, '', data, flags=re.DOTALL)

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(data)
