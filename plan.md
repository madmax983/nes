1. **Update `Cpu::new` in `crates/nes-core/src/cpu/engine.rs`**
   - Use `replace_with_git_merge_diff` to modify `crates/nes-core/src/cpu/engine.rs`.
   - Use `Vec::with_capacity(8)` for `writes`, `prg_writes`, `mmio_reads`, and `bus_trace` to prevent reallocation during the clear-and-swap pattern.
   - Add `//` regular comments explaining that pre-allocating avoids multiple heap reallocations and latency spikes during the initial frames before the capacity stabilizes.
2. **Update `NesCore::new` in `crates/nes-core/src/api.rs`**
   - Use `replace_with_git_merge_diff` to modify `crates/nes-core/src/api.rs`.
   - Use `Vec::with_capacity(8)` for `last_cpu_bus_trace`, `scratch_writes`, and `scratch_mmio_reads` for the same reason.
   - Add `//` regular comments explaining the optimization.
3. **Verify Changes**
   - Read the modified files using `cat` in `run_in_bash_session` to ensure the changes are applied correctly.
4. **Run formatting, linting, and tests**
   - Use `run_in_bash_session`.
   - Run `cargo fmt --all`.
   - Run `cargo clippy --all-targets --all-features -- -D warnings`.
   - Run `cargo test --all-features`.
Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.
6. **Submit PR**
   - Use `run_in_bash_session` to create the final commit and PR submission.
   - Write the commit message with a bash heredoc (e.g., `cat << 'EOF' > commit_msg.txt` and `git commit -F`). The title should be `⚡ Bolt: Pre-allocate hot path buffers`.
   - The description should include the required sections `💡 What`, `🎯 Why`, `📊 Impact`, `🔭 Measurement` as requested in the issue.
