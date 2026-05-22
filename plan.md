1. **Verify workspace changes in `crates/nes-desktop/src/rta.rs`**:
   - Use `run_in_bash_session` to run `git status` and `git diff --staged` to verify the `find_map` optimization that was already completed during exploration.
   - The changes replace an `iter_mut().find_map(...)` iterator chain with a raw `for` loop in `RtaManager::trigger_fired`.

2. **Verify tests and benchmarks pass locally**:
   - Use `run_in_bash_session` to execute `cargo fmt --all`.
   - Use `run_in_bash_session` to execute `cargo clippy --all-targets --all-features -- -D warnings`.
   - Use `run_in_bash_session` to execute `cargo test --all-features`.
   - Use `run_in_bash_session` to execute `cargo bench --bench frame_throughput` to verify performance.

3. **Complete pre commit steps**:
   - Complete pre-commit steps to ensure proper testing, verification, review, and reflection are done.

4. **Submit PR**:
   - Use the `submit` tool to create the PR on branch `jules-876179410565850919-eb3e9999`.
   - Format PR as required by Bolt:
     - Title: "⚡ Bolt: Replace find_map with for loop in trigger evaluation"
     - Description:
       - 💡 What: Replaced an `iter_mut().find_map(...)` iterator chain with a raw `for` loop in `RtaManager::trigger_fired`.
       - 🎯 Why: `trigger_fired` is evaluated multiple times per frame for every possible trigger slot. Removing the iterator adapter chain eliminates the overhead of closure allocation and `Option` wrapping on this highly sensitive hot path.
       - 📊 Impact: Eliminates closure state allocations and wrapper struct instantiation per frame per split trigger, reducing CPU overhead during speedrun evaluation.
       - 🔬 Measurement: Run `cargo bench --bench frame_throughput` and verify `cargo test --all-features` passes.
