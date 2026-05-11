1. Modify `OverlayModel::handle_key` in `crates/nes-desktop/src/overlay.rs` to take ownership of `add_cheat_input` buffer and use `std::mem::take` to extract the `String` into `OverlayCommand::SubmitCheatCode` without cloning. This avoids string heap allocations when submitting cheats.
2. Verify the changes using `run_in_bash_session` with `git diff`.
3. Run all tests to ensure correctness using `run_in_bash_session` with `cargo test --all-features`.
4. Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
5. Submit the PR using the `submit` tool with branch "bolt-rta-manager-allocations", and title "⚡ Bolt: [performance improvement]". The description should include `💡 What`, `🎯 Why`, `📊 Impact`, and `🔬 Measurement`.
