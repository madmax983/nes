1. **Refactor `Mmc3State` restoration**
   - The file `crates/nes-core/src/mapper/mmc3.rs` has already been updated. Ensure the function signature is `pub(crate) fn restore_state(&mut self, state: &Mmc3State)` and `prg_ram` is copied or resized carefully without unnecessary reallocation.
2. **Refactor `Mmc4State` restoration**
   - The file `crates/nes-core/src/mapper/mmc4.rs` has already been updated. Ensure the function signature is `pub(crate) fn restore_state(&mut self, state: &Mmc4State)` and `prg_ram` is copied or resized carefully.
3. **Refactor `Mmc5State` restoration**
   - The file `crates/nes-core/src/mapper/mmc5.rs` has already been updated. Ensure the function signature is `pub(crate) fn restore_state(&mut self, state: &Mmc5State)` and `prg_ram`/`exram` is copied or resized carefully.
4. **Refactor `Fme7State` restoration**
   - The file `crates/nes-core/src/mapper/fme7.rs` has already been updated. Ensure the function signature is `pub(crate) fn restore_state(&mut self, state: &Fme7State)` and `wram` is copied or resized carefully.
5. **Update `apply_delta` in `api.rs`**
   - The file `crates/nes-core/src/api.rs` has already been updated. Ensure `apply_delta` passes references instead of clones for `Mmc3`, `Mmc4`, `Mmc5`, and `Fme7`.
6. **Workspace Validation**
   - Run `cargo fmt --all`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --workspace --all-features` to ensure no functionality is broken.
7. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
8. **Submit the PR**
   - Submit the PR with standard Bolt formatting.
