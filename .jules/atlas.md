# Atlas Journal

**Extract Input Logic from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` was bloated and contained input event classification logic (`classify_window_event`, `classify_keyboard_input`), associated structs/enums (`WindowEventDecision`, `KeyboardDecision`, `FrameDecision`, `KeyboardInputMode`), and timeframe math (`evaluate_frame_deadline`) mixed alongside the core UI event loop. This violated domain boundaries and high cohesion.
**Blueprint:** Extracted the input types and classification functions into a dedicated `crates/nes-desktop/src/input.rs` internal module. Updated `main.rs` to register the new module via `pub(crate) mod input;` and import its contents, significantly reducing `main.rs` file size and separating concerns.


**Extract Input Logic from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` was bloated and contained input event classification logic (`classify_window_event`, `classify_keyboard_input`), associated structs/enums (`WindowEventDecision`, `KeyboardDecision`, `FrameDecision`, `KeyboardInputMode`), and timeframe math (`evaluate_frame_deadline`) mixed alongside the core UI event loop. This violated domain boundaries and high cohesion.
**Blueprint:** Extracted the input types and classification functions into a dedicated `crates/nes-desktop/src/input.rs` internal module. Updated `main.rs` to register the new module via `pub(crate) mod input;` and import its contents, significantly reducing `main.rs` file size and separating concerns.

**Extracted App Modules from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` was massively bloated with over 2000 lines because it housed internal modules like `config`, `gamepad`, `input`, `metrics`, `session`, `mcp_host`, and `netplay` as `pub(crate) mod` declarations. These inner modules leaked binary boundaries and coupled application logic with the core run loop.
**Blueprint:** Converted these internal binary modules into shared library modules by declaring them as `pub mod` inside `crates/nes-desktop/src/lib.rs`. Updated `main.rs` to import them cleanly via `use nes_desktop::...` rather than `use crate::...`. This enforces strict boundaries between library domain types and the binary execution context, reducing sprawl.
