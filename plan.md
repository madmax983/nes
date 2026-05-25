1. **Extract input functions from `main.rs` to `input.rs`**
   - Execute a Python script that finds the input functions `update_button_bits`, `track_keyboard_bits_for_key`, and `merge_local_input_bits` from `crates/nes-desktop/src/main.rs`.
   - Remove these functions from `main.rs` and append them to `input.rs` as `pub(crate) fn`.
   - Extract the tests `update_button_bits_sets_and_clears_masks` and `track_keyboard_bits_for_key_updates_controller_bits_and_ignores_hotkeys` from `main.rs`, remove them, and insert them into the existing `mod tests` block in `input.rs`.
   - Wait, `map_virtual_keycode_maps_all_supported_keys` from `main.rs` is testing `map_virtual_keycode`, which belongs to `input.rs`. Let's also extract this test to `input.rs` (which already has a `map_virtual_keycode_maps_all_keys` test, wait, wait, the one in main.rs might be redundant but I will extract it and rename or just extract it anyway).
   - This script will be:
   ```bash
   cat << 'PYEOF' > move_input.py
import re
with open("crates/nes-desktop/src/main.rs", "r") as f:
    data = f.read()
funcs = ["update_button_bits", "track_keyboard_bits_for_key", "merge_local_input_bits"]
extracted_funcs = []
for func in funcs:
    pattern = r'fn ' + func + r'\s*\([^)]*\)(?:\s*->\s*[^{]+)?\s*\{.*?\n\}\n'
    match = re.search(pattern, data, re.DOTALL)
    if match:
        extracted = match.group(0).replace(f"fn {func}", f"pub(crate) fn {func}")
        extracted_funcs.append(extracted)
        data = data.replace(match.group(0), "")
tests = ["update_button_bits_sets_and_clears_masks", "track_keyboard_bits_for_key_updates_controller_bits_and_ignores_hotkeys", "map_virtual_keycode_maps_all_supported_keys"]
extracted_tests = []
for test in tests:
    pattern = r'(?:#\[test\]\s*)?(?:#\[allow\(deprecated\)\]\s*)?fn ' + test + r'\s*\(\)\s*\{.*?\n    \}\n'
    match = re.search(pattern, data, re.DOTALL)
    if match:
        extracted_tests.append(match.group(0))
        data = data.replace(match.group(0), "")
with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(data)
with open("crates/nes-desktop/src/input.rs", "r") as f:
    input_data = f.read()
input_data = input_data + "\n" + "\n".join(extracted_funcs)
test_insertion = "\n    ".join(extracted_tests).replace("\n", "\n    ")
input_data = re.sub(r'(mod tests \{.*?)\n\}', r'\1\n    ' + test_insertion + r'\n}', input_data, flags=re.DOTALL)
with open("crates/nes-desktop/src/input.rs", "w") as f:
    f.write(input_data)
PYEOF
   python3 move_input.py
   ```

2. **Verify input functions move**
   - Run `git diff crates/nes-desktop/src/input.rs crates/nes-desktop/src/main.rs`.

3. **Extract gamepad functions from `main.rs` to `gamepad.rs`**
   - Execute a Python script to extract gamepad functions `release_all_buttons`, `resync_restored_inputs`, `is_player_two_slot`, `apply_gamepad_delta_commands`, `connected_gamepad_ids`, and `gamepad_snapshot_to_bits` from `crates/nes-desktop/src/main.rs`.
   - Remove these functions from `main.rs` and append them to `gamepad.rs` as `pub(crate) fn`.
   - Extract the tests `gamepad_source_helpers_select_connected_ids_without_duplicates`, `gamepad_sampling_helpers_map_buttons_and_axis_thresholds`, `resync_restored_inputs_reapplies_keyboard_and_resets_gamepad_cache`, `apply_gamepad_delta_commands_updates_controller_bits`, `gamepad_assignment_helpers_detect_global_and_slot_level_changes` and the helper `fake_gamepad_id` from `main.rs`.
   - Append these tests into a new `mod tests` block at the end of `gamepad.rs` because `gamepad.rs` currently lacks one.
   - This script will be:
   ```bash
   cat << 'PYEOF' > move_gamepad.py
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
PYEOF
   python3 move_gamepad.py
   ```

4. **Verify gamepad functions move**
   - Run `git diff crates/nes-desktop/src/gamepad.rs crates/nes-desktop/src/main.rs`.

5. **Fix imports in `main.rs`**
   - Execute a Python script that removes unused imported test functions in `main.rs` (like `apply_gamepad_delta_commands`, `merge_local_input_bits`, etc.) and adds `use crate::input::*;` and `use crate::gamepad::*;` where the modules are declared.
   - This script will be:
   ```bash
   cat << 'PYEOF' > fix_imports.py
import re
with open("crates/nes-desktop/src/main.rs", "r") as f:
    data = f.read()
items_to_remove = ["apply_gamepad_delta_commands", "is_player_two_slot", "merge_local_input_bits", "resync_restored_inputs", "track_keyboard_bits_for_key", "update_button_bits", "connected_gamepad_ids", "gamepad_snapshot_to_bits", "map_virtual_keycode"]
for item in items_to_remove:
    data = re.sub(item + r',\s*', '', data)
if "use crate::input::*;" not in data:
    data = data.replace("pub(crate) mod input;\n", "pub(crate) mod input;\nuse crate::input::*;\n")
if "use crate::gamepad::*;" not in data:
    data = data.replace("pub(crate) mod gamepad;\n", "pub(crate) mod gamepad;\nuse crate::gamepad::*;\n")
with open("crates/nes-desktop/src/main.rs", "w") as f:
    f.write(data)
PYEOF
   python3 fix_imports.py
   ```

6. **Verify import fixes**
   - Run `git diff crates/nes-desktop/src/main.rs`.

7. **Run Linter, Formatter, and Tests**
   - Execute `cargo clippy --all-targets --all-features -- -D warnings` and `cargo fmt --all` and `cargo test`.

8. **Complete pre-commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

9. **Submit the PR**
   - Call the `submit` tool to finalize the change.
