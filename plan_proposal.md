1. **Extract gamepad-related logic into a new module.**
   - Create `crates/nes-desktop/src/gamepad.rs` and move:
     - `GamepadSnapshot` struct
     - `connected_gamepad_ids` function
     - `select_active_gamepad_ids` function
     - `gamepad_snapshot_to_bits` function
     - `controller_state_delta_for_player` function
     - `apply_gamepad_delta_commands` function
     - `gamepad_assignments_changed` function
     - `gamepad_slot_changed` function
     - `resync_restored_inputs` function
     - `release_all_buttons` function
     - `track_keyboard_bits_for_key` function
     - `update_button_bits` function
     - `merge_local_input_bits` function
     - Their associated unit tests from `main.rs`
2. **Expose the new module.**
   - Add `pub(crate) mod gamepad;` to `crates/nes-desktop/src/main.rs`.
3. **Update imports.**
   - In `main.rs`, remove the extracted items and add `use crate::gamepad::*;`. Update any necessary struct visibilities if needed.
4. **Complete pre-commit steps.**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
5. **Create a Pull Request.**
   - Format the PR with the title "🗺️ Atlas: Extract gamepad module from main.rs" and include required sections (Tangle, Blueprint, Stability, Verification).
