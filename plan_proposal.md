1. **Extract `RuntimeConfig`**
   - In `crates/nes-desktop/src/main.rs`, the `RuntimeConfig` struct and its related functions like `resolve_runtime_config` are located, creating bloat in `main.rs`.
   - Create a new module `crates/nes-desktop/src/config.rs`.
   - Move `RuntimeConfig`, `StepMode`, and `CaptureConfig` into `config.rs`, making their fields `pub`.
   - Move `resolve_runtime_config`, `capture_config_from_parts`, and `capture_path_for_frame` (if applicable) into `config.rs`.
   - Add `pub mod config;` to `crates/nes-desktop/src/lib.rs`.
   - Update imports in `main.rs`.

2. **Verify changes**
   - Run `cargo check`, `cargo test`, and `cargo fmt --all`.

3. **Complete pre-commit steps**
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
