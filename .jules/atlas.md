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
**Extract Frame Capture Logic from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` contained frame capture and encoding logic (`should_capture_frame`, `capture_path_for_frame`, `write_frame_ppm`, `encode_ppm`). This logic is distinct from the core UI event loop, mixing image encoding and file I/O with application orchestration.
**Blueprint:** Extracted the frame capture logic into a dedicated `crates/nes-desktop/src/capture.rs` internal module. Updated `main.rs` to register the new module via `pub(crate) mod capture;` and import its contents, improving separation of concerns and file size.
