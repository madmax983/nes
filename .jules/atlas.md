**Extract Input Boundary from API**
**Tangle:** The `api.rs` module in `nes-core` contained a lot of internal controller state logic mixed with public API endpoints (`Button`, `Player`, `ControllerState`, `ControllerPorts`).
**Blueprint:** Extracted the controller domain models (`Button`, `Player`, `ControllerState`, `ControllerPorts`) into a new internal module `crates/nes-core/src/input.rs`, breaking the bloat and maintaining domain cohesion, while re-exporting `Button` and `Player` to keep the public facade intact.
**Extract Input Boundary from API**
**Tangle:** The `api.rs` module in `nes-core` contained a lot of internal controller state logic mixed with public API endpoints (`Button`, `Player`, `ControllerState`, `ControllerPorts`).
**Blueprint:** Extracted the controller domain models (`Button`, `Player`, `ControllerState`, `ControllerPorts`) into a new internal module `crates/nes-core/src/input.rs`, breaking the bloat and maintaining domain cohesion, while re-exporting `Button` and `Player` to keep the public facade intact.
