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
