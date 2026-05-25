with open("crates/nes-desktop/src/main.rs", "r") as f:
    data = f.read()

# I removed the functions from main.rs but forgot to delete them from main tests!
# wait, the first error is `error[E0432]: unresolved imports super::apply_gamepad_delta_commands ...` which means they are being imported inside the tests of main.rs but they are no longer in the main root module.

import re

# Remove `apply_gamepad_delta_commands`, `is_player_two_slot`, `merge_local_input_bits`, `resync_restored_inputs`, `track_keyboard_bits_for_key`, `update_button_bits`, `map_virtual_keycode`, `connected_gamepad_ids`, `gamepad_snapshot_to_bits` from tests import list

items_to_remove = ["apply_gamepad_delta_commands", "is_player_two_slot", "merge_local_input_bits", "resync_restored_inputs", "track_keyboard_bits_for_key", "update_button_bits", "map_virtual_keycode", "connected_gamepad_ids", "gamepad_snapshot_to_bits"]

for item in items_to_remove:
    data = re.sub(item + r',\s*', '', data)

# Also fix the unused import `use nes_desktop::app::map_key_event_to_button_bit;` which we moved to input.rs instead of main.rs? Wait, it's unused in main.rs because input.rs needs it and main.rs no longer does.
data = re.sub(r'use nes_desktop::app::map_key_event_to_button_bit;\n', '', data)

with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(data)
