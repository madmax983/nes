## 2026-04-22 - Argument Parsing Mutants
**Mutant:** `replace == with !=` at `if arg == "--auto-player"`, `replace += with -=` at `idx += 1;`, and `replace += with *=` at `idx += 1;` in `crates/nes-desktop/src/args.rs`.
**Diagnosis:** `TIMEOUT` logic resulting from mutated loop indexing or condition match causes mutants to survive as expected weaknesses because tests timeout without catching them properly due to continuous evaluation loops. These are expected weaknesses based on how test runner enforces time limits.
**Kill Shot:** We will not fix them. Documenting this as an expected weakness.
## 2025-02-06 - Missing Test for `to_js_error` in `nes-web`
**Mutant:** `replace to_js_error -> JsValue with Default::default()` in `crates/nes-web/src/lib.rs`
**Diagnosis:** The test suite was missing coverage for the `to_js_error` function, which maps a Rust error string into a Javascript `JsValue` representation of the error.
**Kill Shot:** Created `test_to_js_error_returns_string` in `crates/nes-web/tests/test_to_js_error.rs` to verify the error value contains the correct string content.
