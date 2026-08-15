# Atlas Journal

**Extract Input Logic from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` was bloated and contained input event classification logic (`classify_window_event`, `classify_keyboard_input`), associated structs/enums (`WindowEventDecision`, `KeyboardDecision`, `FrameDecision`, `KeyboardInputMode`), and timeframe math (`evaluate_frame_deadline`) mixed alongside the core UI event loop. This violated domain boundaries and high cohesion.
**Blueprint:** Extracted the input types and classification functions into a dedicated `crates/nes-desktop/src/input.rs` internal module. Updated `main.rs` to register the new module via `pub(crate) mod input;` and import its contents, significantly reducing `main.rs` file size and separating concerns.


**Extract Input Logic from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` was bloated and contained input event classification logic (`classify_window_event`, `classify_keyboard_input`), associated structs/enums (`WindowEventDecision`, `KeyboardDecision`, `FrameDecision`, `KeyboardInputMode`), and timeframe math (`evaluate_frame_deadline`) mixed alongside the core UI event loop. This violated domain boundaries and high cohesion.
**Blueprint:** Extracted the input types and classification functions into a dedicated `crates/nes-desktop/src/input.rs` internal module. Updated `main.rs` to register the new module via `pub(crate) mod input;` and import its contents, significantly reducing `main.rs` file size and separating concerns.
**Remove Duplicated input methods in main.rs**
**Tangle:** The `map_virtual_keycode` method was duplicated in `main.rs` but it already existed and had been moved to `input.rs` in an earlier refactor (as mentioned in a previous journal entry).
**Blueprint:** Removed the duplicated `map_virtual_keycode` from `crates/nes-desktop/src/main.rs`.
**Extract AppContext boilerplate in main.rs**
**Tangle:** The `crates/nes-desktop/src/main.rs` file was cluttered with 5 occurrences of the complex struct initialization for `AppContext`. This made the `event_loop.run` closure bloated and repetitive, violating DRY principles. Extracting it to a function is difficult because it borrows 15+ local mutable variables within the same scope.
**Blueprint:** Replaced the struct instantiations with a locally scoped `macro_rules! build_ctx!()` that implicitly captures all the local variables, reducing boilerplate while avoiding lifetime/borrowing issues.

**Extract Constants from api.rs**
**Tangle:** A circular dependency existed in `nes-core` where `apu.rs` and `ppu.rs` (internal modules) imported domain constants (`FRAME_WIDTH`, `AUDIO_SAMPLE_RATE`, etc.) from the public facade module `api.rs`.
**Blueprint:** Extracted the domain constants into a new internal `constants.rs` module, breaking the circular dependency while continuing to re-export the constants via `lib.rs` for external consumers.

**Extract remaining Input/Gamepad Logic from main.rs**
**Tangle:** The `main.rs` file still contained several helper functions related to input and gamepad management, including `track_keyboard_bits_for_key`, `update_button_bits`, `merge_local_input_bits`, `map_virtual_keycode`, `release_all_buttons`, `is_player_two_slot`, `apply_gamepad_delta_commands`, and `resync_restored_inputs`, as well as their tests. These functions made `main.rs` bloated and violated single-responsibility boundaries.
**Blueprint:** Moved keyboard and input bit masking logic (`update_button_bits`, `track_keyboard_bits_for_key`, `merge_local_input_bits`, `map_virtual_keycode`) into `crates/nes-desktop/src/input.rs`. Moved gamepad and command generation logic (`release_all_buttons`, `is_player_two_slot`, `apply_gamepad_delta_commands`, `resync_restored_inputs`) into `crates/nes-desktop/src/gamepad.rs`. The code was cleaned up and unit tests for each group of helpers were relocated accordingly. `main.rs` now properly uses the components via `crate::input::*` and `crate::gamepad::*`, resulting in a much cleaner, more cohesive UI event loop.
**Remove Duplicated map_virtual_keycode in main.rs**
**Tangle:** The `map_virtual_keycode` method in `nes-desktop` was duplicated. It existed both in the newly created `input.rs` and in `main.rs`. This duplicated logic which could go out of sync and made the binary module unnecessarily large.
**Blueprint:** Removed the duplicated `map_virtual_keycode` from `main.rs` since it was already correctly placed in the `input.rs` module and being utilized properly from there.
**Extract Mapper Dispatch from api.rs**
**Tangle:** The `crates/nes-core/src/api.rs` file was a massive blob containing the entire core facade as well as internal mapper dispatch logic (e.g., `LoadedMapper`, `MapperDelta`, `MapperDeltaKind` enums). This forced the public API module to depend on every internal mapper struct (like `Mmc3State`), breaking single responsibility and making the file over 3000 lines long.
**Blueprint:** Extracted the mapper dispatch enums and implementations into `crates/nes-core/src/mapper/dispatch.rs`. Re-exported them via `crates/nes-core/src/mapper/mod.rs` (`pub use dispatch::MapperDelta`). This significantly reduced `api.rs` size and isolated the mapper routing logic inside the `mapper` module boundary.
