## 2026-04-22 - Argument Parsing Mutants
**Mutant:** `replace == with !=` at `if arg == "--auto-player"`, `replace += with -=` at `idx += 1;`, and `replace += with *=` at `idx += 1;` in `crates/nes-desktop/src/args.rs`.
**Diagnosis:** `TIMEOUT` logic resulting from mutated loop indexing or condition match causes mutants to survive as expected weaknesses because tests timeout without catching them properly due to continuous evaluation loops. These are expected weaknesses based on how test runner enforces time limits.
**Kill Shot:** We will not fix them. Documenting this as an expected weakness.

## 2026-04-22 - Infinite Loop Timeouts in nes-config and nes-rewind
**Mutant:** `replace += with -=` and `replace += with *=` at `idx += 1;` in `crates/nes-config/src/lib.rs` (`parse_arg` and `parse_config_path_arg`). Also `replace += with *=` in `diff_array` in `crates/nes-rewind/src/delta.rs`.
**Diagnosis:** `TIMEOUT` logic resulting from mutated loop incrementers causing tests to run infinitely. These are expected weaknesses based on how the test runner enforces time limits on tight loops.
**Kill Shot:** Documenting this as an expected weakness since `cargo mutants` interprets it as a timeout.
