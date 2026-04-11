# Atlas Journal

**Extract Input Logic from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` was bloated and contained input event classification logic (`classify_window_event`, `classify_keyboard_input`), associated structs/enums (`WindowEventDecision`, `KeyboardDecision`, `FrameDecision`, `KeyboardInputMode`), and timeframe math (`evaluate_frame_deadline`) mixed alongside the core UI event loop. This violated domain boundaries and high cohesion.
**Blueprint:** Extracted the input types and classification functions into a dedicated `crates/nes-desktop/src/input.rs` internal module. Updated `main.rs` to register the new module via `pub(crate) mod input;` and import its contents, significantly reducing `main.rs` file size and separating concerns.


**Extract Input Logic from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` was bloated and contained input event classification logic (`classify_window_event`, `classify_keyboard_input`), associated structs/enums (`WindowEventDecision`, `KeyboardDecision`, `FrameDecision`, `KeyboardInputMode`), and timeframe math (`evaluate_frame_deadline`) mixed alongside the core UI event loop. This violated domain boundaries and high cohesion.
**Blueprint:** Extracted the input types and classification functions into a dedicated `crates/nes-desktop/src/input.rs` internal module. Updated `main.rs` to register the new module via `pub(crate) mod input;` and import its contents, significantly reducing `main.rs` file size and separating concerns.

**Extract AppContext from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` contained a large amount of core action execution logic and the God struct `AppContext` which resulted in an overly long and coupled file.
**Blueprint:** Extracted the `AppContext` struct and functions related to action dispatching (`dispatch_app_action`, `dispatch_overlay_command`, `execute_app_action`, `set_overlay_open`, `reconcile_core_pause_with_overlay`, `validate_action_allowed`, and `command_marks_rta_invalidation`) into a new `crates/nes-desktop/src/app_context.rs` internal module, reducing file size and improving domain boundary separation.
