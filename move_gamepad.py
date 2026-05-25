import re
with open("crates/nes-desktop/src/main.rs", "r") as f:
    data = f.read()
funcs = ["release_all_buttons", "resync_restored_inputs", "is_player_two_slot", "apply_gamepad_delta_commands", "connected_gamepad_ids", "gamepad_snapshot_to_bits"]
extracted_funcs = []
for func in funcs:
    pattern = r'fn ' + func + r'\s*\([^)]*\)(?:\s*->\s*[^{]+)?\s*\{.*?\n\}\n'
    match = re.search(pattern, data, re.DOTALL)
    if match:
        extracted = match.group(0).replace(f"fn {func}", f"pub(crate) fn {func}")
        extracted_funcs.append(extracted)
        data = data.replace(match.group(0), "")
tests = ["resync_restored_inputs_reapplies_keyboard_and_resets_gamepad_cache", "apply_gamepad_delta_commands_updates_controller_bits", "gamepad_source_helpers_select_connected_ids_without_duplicates", "gamepad_sampling_helpers_map_buttons_and_axis_thresholds", "gamepad_assignment_helpers_detect_global_and_slot_level_changes"]
extracted_tests = []
for test in tests:
    pattern = r'(?:#\[test\]\s*)?(?:#\[allow\(deprecated\)\]\s*)?fn ' + test + r'\s*\(\)\s*\{.*?\n    \}\n'
    match = re.search(pattern, data, re.DOTALL)
    if match:
        extracted_tests.append(match.group(0))
        data = data.replace(match.group(0), "")
match = re.search(r'fn fake_gamepad_id\s*\([^)]*\)(?:\s*->\s*[^{]+)?\s*\{.*?\n    \}\n', data, re.DOTALL)
if match:
    extracted_tests.insert(0, match.group(0))
    data = data.replace(match.group(0), "")
with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(data)
with open("crates/nes-desktop/src/gamepad.rs", "r") as f:
    gamepad_data = f.read()
gamepad_data = gamepad_data + "\n" + "\n".join(extracted_funcs)
gamepad_data += "\n\n#[cfg(test)]\nmod tests {\n    use super::*;\n    use gilrs::GamepadId;\n    use nes_core::NesCore;\n\n" + "\n".join(extracted_tests) + "\n}"
with open("crates/nes-desktop/src/gamepad.rs", "w") as f:
    f.write(gamepad_data)
