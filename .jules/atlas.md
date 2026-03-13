# Atlas Journal

**Refactor `main.rs` in `nes-desktop`**
**Tangle:** The `main.rs` file was very large and contained netplay-specific logic mixed with core desktop concerns.
**Blueprint:** Extracted `NetplayRuntimeStats` and associated helper functions (like `handle_netplay_server_message`, `schedule_netplay_ping`, `compute_local_netplay_bits`, `should_send_netplay_hash`) along with their unit tests from `main.rs` and moved them to `netplay.rs`. Updated imports and function calls to use `crate::netplay::*`.
