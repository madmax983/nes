## 2026-04-22 - Argument Parsing Mutants
**Mutant:** `replace == with !=` at `if arg == "--auto-player"`, `replace += with -=` at `idx += 1;`, and `replace += with *=` at `idx += 1;` in `crates/nes-desktop/src/args.rs`.
**Diagnosis:** `TIMEOUT` logic resulting from mutated loop indexing or condition match causes mutants to survive as expected weaknesses because tests timeout without catching them properly due to continuous evaluation loops. These are expected weaknesses based on how test runner enforces time limits.
**Kill Shot:** We will not fix them. Documenting this as an expected weakness.
