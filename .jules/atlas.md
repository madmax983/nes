# Atlas Journal

**Refactor `main.rs` in `nes-desktop`**
**Tangle:** The `main.rs` file was very large and contained netplay-specific logic mixed with core desktop concerns.
**Blueprint:** Extracted `NetplayRuntimeStats` and associated helper functions (like `handle_netplay_server_message`, `schedule_netplay_ping`, `compute_local_netplay_bits`, `should_send_netplay_hash`) along with their unit tests from `main.rs` and moved them to `netplay.rs`. Updated imports and function calls to use `crate::netplay::*`.
**Extract RtaRuntimeConfig**
**Tangle:** The `RtaRuntimeConfig` struct was defined inside `nes-desktop/src/main.rs`, but conceptually it belongs with the `rta` module alongside other RTA-related structs.
**Blueprint:** Moved `RtaRuntimeConfig` to `nes-desktop/src/rta.rs` and marked its fields `pub`, then imported it into `main.rs`.

**Extract PerfMetrics from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` was bloated and contained highly cohesive logic for metrics tracking (`PerfMetrics`, `MetricsSnapshot`) mixed with core desktop/windowing logic.
**Blueprint:** Extracted the metrics tracking structures and functions into `crates/nes-desktop/src/metrics.rs` and exposed them via `pub(crate)` boundaries.

**Refactoring NesCore Controller State**
**Tangle:** The `NesCore` struct and `api.rs` implementation block had become bloated with low-level controller port state (`controllers: [ControllerState; 2]`, `controller_strobe: bool`) and its associated manipulation methods mixed alongside high-level system components. This reduced cohesion within `NesCore` and violated encapsulation.
**Blueprint:** Extracted the controller state into a dedicated `ControllerPorts` internal struct. Relocated `set_controller_bits`, `write_controller_strobe`, `controller_port_sample`, and `consume_controller_read` methods to `impl ControllerPorts`, simplifying the `NesCore` implementation block and consolidating input logic.

**Refactor nes-test-harness test support code into library**
**Tangle:** Shared ROM path helpers for integration tests were defined in `tests/support/mod.rs`, which violates best practices by duplicating code and relying on implicit modules in tests.
**Blueprint:** Moved the `tests/support/mod.rs` file into the `nes-test-harness` library as `src/rom_paths.rs`, exposing it publicly. Updated all test files to import the functions directly from the `nes_test_harness` crate, improving encapsulation and reusability.

**Extract Audio Handling from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` contained the internal structures for audio output (`AudioSinkControl`, `RodioSinkAdapter`, `AudioOutput`) alongside fake implementations and test cases mixed with UI loop and generic app state.
**Blueprint:** Extracted all audio structures, helper implementations, and their dedicated tests into `crates/nes-desktop/src/audio.rs` and exposed them internally via `pub(crate) mod audio`. Updated `main.rs` to import from the new module instead.

**Extract Gamepad Logic from main.rs**
**Tangle:** The `main.rs` file in `nes-desktop` contained several gamepad-specific structs (`GamepadSnapshot`), constants (`CONTROLLER_BUTTONS`, `GAMEPAD_AXIS_THRESHOLD`), and helper methods mapping external inputs to internal NES core bits mixed in with generic UI state logic.
**Blueprint:** Extracted all gamepad translation logic and related structs/constants into a new internal module `crates/nes-desktop/src/gamepad.rs`. Registered the module in `main.rs` as `pub(crate) mod gamepad;` to maintain strict boundary isolation from the public API while reducing `main.rs` bloat.

**Extract Keyboard Input Classification from main.rs**
**Tangle:** The `main.rs` file contained low-level keyboard input mapping and decision classification logic (`KeyboardDecision`, `KeyboardInputMode`, `classify_keyboard_input`, `map_virtual_keycode`) mixed with the core desktop UI event loop.
**Blueprint:** Extracted the keyboard input classification logic and its unit tests into `crates/nes-desktop/src/app.rs` and exposed them via `pub`.
