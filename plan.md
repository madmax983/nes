1. **Extract Input/Gamepad functions from `main.rs` to `input.rs` and `gamepad.rs`**
   - Move `update_button_bits`, `track_keyboard_bits_for_key`, and `merge_local_input_bits` from `main.rs` to `input.rs` (and make them `pub(crate)`).
   - Move `release_all_buttons`, `resync_restored_inputs`, `is_player_two_slot`, and `apply_gamepad_delta_commands` from `main.rs` to `gamepad.rs` (and make them `pub(crate)`).
   - Move the corresponding unit tests from `main.rs`'s test block to `input.rs` and `gamepad.rs`.

2. **Update imports in `main.rs`**
   - Update `main.rs` to import the moved functions from `crate::input` and `crate::gamepad`.

3. **Complete pre commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

4. **Submit the PR**
