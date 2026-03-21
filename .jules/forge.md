**Refactoring `LoadedMapper` and `NesCore` API**
**Learning:** Found deep nested `if let` matching in `apply_delta` which created unneeded pyramids, and discovered "boolean blindness" in controller state functions (`player2: bool` arguments). Also found `match` statements across enums returning `Ok(())` in every block.
**Action:** Replaced `player2: bool` with `pub(crate) enum Player { One, Two }`. Rewrote `apply_delta` to use `let ... else` guard clauses to break deep nesting. Rewrote the `execute` match block to evaluate to `()` and return a single `Ok(())` at the end to drastically simplify repetitive code.

**Refactoring boolean conversions and nested iterators in nes-core**
**Learning:** Found several boolean blindness issues using `if condition { 1 } else { 0 }` instead of idiomatic `From` trait implementations. Also found a case of intermediate Vec allocation during iterator filtering in `apply_cpu_writes` that could be optimized out using `filter`.
**Action:** Replaced boolean conversions with `u64::from` and `i32::from`. Simplified `apply_cpu_writes` by chaining `iter().filter()` to eliminate unneeded variable mutation and nesting.
**Refactoring hardware cycle loops in nes-core API**
**Learning:** Found scattered and repeated logic manually running `self.step_hardware_cycle()` inside `for _ in 0..cycles` loops in `api.rs`, hiding the fact that chained DMC requests handled within `step_hardware_cycle` implicitly recurse back to `apply_dmc_dma_request`.
**Action:** Extracted `advance_hardware_cycles` helper function and replaced manual loops in `step_single_instruction` and `run_oam_dma`. Deliberately avoided refactoring `apply_dmc_dma_request` because its inline loop explicitly handles chained requests without triggering recursive stalls, which is required for accurate hardware timing.

**Refactoring guard clauses in nes-core mapper synchronization**
**Learning:** Found deeply nested `if let` blocks inside mapper synchronization functions (`sync_mapper_prg_window`, `sync_mapper_chr_window`, and `sync_mapper_mirroring`), creating unnecessary indentation and violating early return principles.
**Action:** Replaced nested `if let` and `&& let` bindings with flat `let Some(...) = ... else { return; }` guard clauses to reduce nesting depth and improve readability.
