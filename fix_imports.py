import re
with open("crates/nes-desktop/src/main.rs", "r") as f:
    data = f.read()
items_to_remove = ["apply_gamepad_delta_commands", "is_player_two_slot", "merge_local_input_bits", "resync_restored_inputs", "track_keyboard_bits_for_key", "update_button_bits", "connected_gamepad_ids", "gamepad_snapshot_to_bits"]
for item in items_to_remove:
    data = re.sub(item + r',\s*', '', data)
if "use crate::input::*;" not in data:
    data = data.replace("pub(crate) mod input;\n", "pub(crate) mod input;\nuse crate::input::*;\n")
if "use crate::gamepad::*;" not in data:
    data = data.replace("pub(crate) mod gamepad;\n", "pub(crate) mod gamepad;\nuse crate::gamepad::*;\n")
with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(data)
