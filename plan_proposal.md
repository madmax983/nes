1. **Remove Duplicate Config Logic in `main.rs`**
   - The functions `resolve_runtime_config`, `capture_config_from_parts`, and `netplay_feature_enabled` are still present in `crates/nes-desktop/src/main.rs`.
   - Remove these duplicate functions and structurally ensure `main.rs` is using the ones from `config.rs`.
   - Use `replace_with_git_merge_diff` to delete these segments from `crates/nes-desktop/src/main.rs`.

2. **Verify changes and test**
   - Run the full verification suite: `cargo test --workspace --all-targets --all-features && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all`.

3. **Complete pre commit steps**
   - Complete pre commit steps to ensure proper testing, verification, review, and reflection are done.

4. **Submit the PR**
   - Use `submit` to submit the PR with the branch name `atlas/extract-config`. The PR details will be:
     - Title: 🗺️ Atlas: Extract RuntimeConfig from main.rs
     - Description:
       🕸️ Tangle: The `main.rs` file was extremely bloated, containing configuration parsing and setup mixed directly with the GUI application loop.
       📏 Blueprint: Extracted `RuntimeConfig`, `StepMode`, `CaptureConfig` and the `resolve_runtime_config` setup function into a separate internal `config.rs` module.
       🧱 Stability: Reduced coupling in the `main.rs` module, cleanly separating the system's runtime setup boundary from its core execution boundaries.
       🔬 Verification: Builds successfully, all tests pass, strict separation enforced via `pub(crate)`.
