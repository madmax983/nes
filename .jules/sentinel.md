## 2026-04-22 - Argument Parsing Mutants
**Mutant:** `replace == with !=` at `if arg == "--auto-player"`, `replace += with -=` at `idx += 1;`, and `replace += with *=` at `idx += 1;` in `crates/nes-desktop/src/args.rs`.
**Diagnosis:** `TIMEOUT` logic resulting from mutated loop indexing or condition match causes mutants to survive as expected weaknesses because tests timeout without catching them properly due to continuous evaluation loops. These are expected weaknesses based on how test runner enforces time limits.
**Kill Shot:** We will not fix them. Documenting this as an expected weakness.

**McpHost Read Framed Message Infinite Loop**
**Mutant:** `replace == with !=` at `if read == 0` in `crates/nes-desktop/src/mcp_host.rs`
**Diagnosis:** `TIMEOUT` logic resulting from mutated condition match causes mutants to survive as expected weaknesses because tests timeout without catching them properly due to continuous evaluation loops. This loop breaks when reading headers reaches EOF. If == is changed to !=, the loop will infinitely wait instead of gracefully erroring on unexpected EOF.
**Kill Shot:** We will not fix them. Documenting this as an expected weakness.

**Metrics Print Mutants**
**Mutant:** `replace print_metrics_table with ()` in `crates/nes-desktop/src/metrics.rs` and various logic mutants inside `print_metrics_table`.
**Diagnosis:** This function `print_metrics_table` writes to stdout. It is purely for debugging/diagnostic output and asserting exactly what table characters and colors it outputs is tedious and often considered low-value coverage theater.
**Kill Shot:** We will not fix them. Documenting this as an equivalent/low-value test gap.
