1. **Refactor `execute_app_action` in `crates/nes-desktop/src/main.rs`**
   - The `execute_app_action` function is quite long and handles multiple complex match arms for `AppAction`.
   - I will extract the logic for each specific action (`OpenRom`, `SaveSlot`, `LoadSlot`, `Reset`) into dedicated helper functions: `execute_open_rom`, `execute_save_slot`, `execute_load_slot`, and `execute_reset`.
   - I will use the existing `AppContext` struct to pass the necessary state into these helper functions, preventing the need for long argument lists.

2. **Run formatting and clippy**
   - After the refactoring, I will run `cargo fmt --all` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` to ensure the changes are idiomatic and correctly formatted.

3. **Verify with tests**
   - I will run `cargo test --workspace --all-targets --all-features` to ensure no runtime behavior has changed and all existing tests continue to pass.

4. **Complete pre-commit checks**
   - I will call the `pre_commit_instructions` tool to complete pre commit steps to make sure proper testing, verifications, reviews and reflections are done.

5. **Submit the PR**
   - Create a PR with the title '⚒️ Forge: Extract `execute_app_action` logic' and the required description format: '🚷 Smell', '✨ Solution', '🧱 Benefit', '🛡️ Verification'.
