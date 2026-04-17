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

**Extract Constants from api.rs**
**Tangle:** A circular dependency existed in `nes-core` where `apu.rs` and `ppu.rs` (internal modules) imported domain constants (`FRAME_WIDTH`, `AUDIO_SAMPLE_RATE`, etc.) from the public facade module `api.rs`.
**Blueprint:** Extracted the domain constants into a new internal `constants.rs` module, breaking the circular dependency while continuing to re-export the constants via `lib.rs` for external consumers.
