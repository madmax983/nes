1. **Analyze surviving mutants in `crates/nes-core/src/tas.rs`:**
   - `<impl fmt::Display for TasError>::fmt -> fmt::Result with Ok(Default::default())`: Missing test for `TasError::fmt()`.
   - `TasMovie::from_runs -> Self with Default::default()`: Missing test verifying that `from_runs` properly initializes from a vector of runs.
   - `TasMovie::runs`: Missing tests verifying the returned slice matches exactly.
   - `TasMovie::total_frames`: Missing test verifying edge cases like `0` or `1` runs.
   - `TasMovie::replay`: Missing tests checking the return value and behavior of `frames_elapsed += 1`.
   - `TasMovie::to_macro_script` & `TasRecorder::macro_script` / `append_button_transitions` / `append_wait`: Tests covering macro script generation with buttons pressed/released are completely missing!
   - `TasMovie::push_run`, `push_frame`: Missing edge case test handling logic like `&&` -> `||`.
   - `TasRecorder`: Missing state toggling verification like `start`, `stop`, `clear`, etc, specifically edge cases.

2. **Categorize the mutants:**
   - Most of them are `MISSING_COVERAGE` or `WEAK_ASSERTION`.
   - The entire `TasMovie::to_macro_script` logic (which translates bits into strings like `PRESS A\nWAIT 1\nRELEASE A\n`) lacks any successful format assertion.
   - The test `test_tas_movie_to_macro_script_fails_with_player_2_input` only tests the `Err` case.

3. **Write the kill shots:**
   - **`test_tas_error_display`**: Assert `TasError::Player2MacroScriptUnsupported.to_string()`.
   - **`test_tas_movie_total_frames`**: Add more assertions to `test_tas_movie_methods`.
   - **`test_tas_movie_to_macro_script_success`**: Test `to_macro_script` with `PRESS`, `RELEASE`, `WAIT`. This will kill a huge chunk of mutants in `to_macro_script`, `append_button_transitions`, and `append_wait`.
   - **`test_tas_recorder_state`**: Test `start`, `stop`, `is_recording` explicitly, asserting `is_recording` doesn't just return `true`/`false`.

4. **Verify the kills:**
   - I will append the tests to the `tests_mutants` module in `crates/nes-core/src/tas.rs`.
   - Re-run `cargo mutants --file crates/nes-core/src/tas.rs --timeout 20 -j 1 -- --manifest-path crates/nes-core/Cargo.toml --features tas tas::tests_mutants::`
   - Complete pre-commit steps.
   - Submit the PR.
