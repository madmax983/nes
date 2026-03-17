**Refactoring `LoadedMapper` and `NesCore` API**
**Learning:** Found deep nested `if let` matching in `apply_delta` which created unneeded pyramids, and discovered "boolean blindness" in controller state functions (`player2: bool` arguments). Also found `match` statements across enums returning `Ok(())` in every block.
**Action:** Replaced `player2: bool` with `pub(crate) enum Player { One, Two }`. Rewrote `apply_delta` to use `let ... else` guard clauses to break deep nesting. Rewrote the `execute` match block to evaluate to `()` and return a single `Ok(())` at the end to drastically simplify repetitive code.

**Refactoring boolean conversions and nested iterators in nes-core**
**Learning:** Found several boolean blindness issues using `if condition { 1 } else { 0 }` instead of idiomatic `From` trait implementations. Also found a case of intermediate Vec allocation during iterator filtering in `apply_cpu_writes` that could be optimized out using `filter`.
**Action:** Replaced boolean conversions with `u64::from` and `i32::from`. Simplified `apply_cpu_writes` by chaining `iter().filter()` to eliminate unneeded variable mutation and nesting.
