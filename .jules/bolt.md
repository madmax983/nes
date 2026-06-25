**[Optimizing map_region in nes-core bus.rs]
**Learning:** `match` on ranges (`0x0000..=0x1FFF => ...`) compiles to exactly the same assembly as `if/else if` chains (`if addr < 0x2000 ...`) due to LLVM optimizations. Additionally, transforming the branch structure into a binary search shape does not improve performance.
**Action:** Do not micro-optimize `match` statements with ranges in Rust when LLVM can optimize them perfectly.

**[Pre-allocating scratch vectors in CPU trace routines]
**Learning:** Initializing `Vec::new()` instead of `Vec::with_capacity(...)` for short-lived traces (e.g. `writes`, `prg_writes`, `mmio_reads`, `bus_trace`) inside `Cpu::new()` and `NesCore::new()` showed statistically insignificant performance differences (actually regressed by a fraction of a percent) during frame throughput tests, since these vectors are cleared and re-used for thousands of frames. The initial allocation happens once per emulator lifespan and is amortized.
**Action:** Avoid micro-optimizing constructor vector capacities unless profiling shows structural reallocation overhead during the hot loop (e.g., inside `step_cpu`).

**[Case-insensitive string comparisons]
**Learning:** `head.to_ascii_uppercase()` allocates a new string just to perform a lookup in a match statement like `matches!(mnemonic, "BCC" | "BCS" ... )` and `opcode_for(&mnemonic, ...)`. This is an unnecessary heap allocation, especially inside parsers or assemblers that run per line. Using `eq_ignore_ascii_case` avoids the allocation but the match statement syntax is simpler.
**Action:** To refactor extensive `if/else if` chains evaluating string case-insensitivity without allocating new strings (e.g., avoiding `.to_ascii_uppercase()`), use a `match` expression with guard clauses (e.g., `_ if s.eq_ignore_ascii_case(...) => ...`).

**[JSON-RPC Parser Allocations]
**Learning:** In `nes-mcp/src/dispatch.rs`, the `ToolParams` type is heavily used to extract strings from a BTreeMap. Creating unnecessary owned `String` variables by cloning values from the BTreeMap just to pass them around inside parsers creates a ton of short-lived heap allocations.
**Action:** In JSON-RPC or similar parsers (e.g., `nes-mcp/src/dispatch.rs` handling `BTreeMap<String, String>`), prefer returning borrowed `&str` references tied to the input map's lifetime rather than using `.cloned()` or allocating new `String`s, deferring `.to_owned()` allocations only to the final structs that explicitly require them.
