🤖 **Sentinel: Closing Test Coverage Gaps in `cheat_codes.rs` and `tas.rs`**

This PR aims to close test gaps identified through `cargo-mutants` where there were previously untested boundary cases or properties.

**🧬 Mutants Found:**
- `cheat_codes.rs`: Mutations substituting bitwise `|` with bitwise `^`. These are **Equivalent Mutants** since the masks ensure distinct bit fields are populated independently without overlap. Documented in journal.
- `cheat_codes.rs`: Missing coverage for checking the specific individual bits parsed from the string and stored in the internal fields.
- `tas.rs`: Missing coverage asserting exactly what data structures `TasFrameRun::new()` and `TasMovie::runs()` output or return. Missing basic interaction tests confirming things like `record_core_frame` logic. Note: Cargo mutants has an issue successfully recognizing the mutations as caught in integration test files directly due to missing or unsupported cargo configuration inside the workspace environment or test isolation. The tests provided strictly enforce the API signatures in the normal cargo test suite.

**🎯 Tests Added/Strengthened:**
- **`cheat_codes.rs`**: Added explicit test verifying that `raw()` returns the normalized string intact. Added explicit failure case tests to handle strings made up solely of hyphens and whitespace.
- **`cheat_codes.rs`**: Added `test_cheat_code_decodes_mutants` to rigidly test bitfields and verify all exact bit states for address and values are extracted correctly.
- **`tas.rs`**: Added basic unit level property tests like `test_tas_movie_methods` and `test_tas_frame_run_new` to verify basic functionality locally within the testing suite.

**⚠️ Suspected Bugs:**
- None detected here.
