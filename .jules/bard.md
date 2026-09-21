## 2025-07-20 - Missing Nova Module Documentation
**Confusion:** The experimental Spatial Bot and PpuVisualizer modules under the `nova` feature were missing Rustdoc comments, which caused `cargo clippy` to fail because `-D warnings` implies `-D missing-docs`.
**Clarification:** Added missing module, struct, field, and function documentation for `SpatialBot`, `BotRule`, and `PpuVisualizer` to satisfy the linter and help users understand how to extract PPU state and automate spatial events.
