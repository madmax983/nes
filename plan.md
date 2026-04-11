1. **Extract `AppContext` and related core models out of `main.rs`**
   - Move `LoadedRomSession`, `RuntimeConfig`, `CaptureConfig`, `AudioOutput` and `AppContext` into a new `context.rs` module in `crates/nes-desktop/src/context.rs`.
   - Update `main.rs` imports to use `crate::context::*`.
   - Modify visibility as needed for fields of `AppContext` and other structs to `pub(crate)`.

2. **Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done**
   - Run `pre_commit_instructions` tool to verify the codebase remains intact and builds successfully.

3. **Update Atlas Journal and Submit**
   - Log the architectural change to `.jules/atlas.md`.
   - Create PR using the `submit` tool with title "🗺️ Atlas: [architectural change]"
