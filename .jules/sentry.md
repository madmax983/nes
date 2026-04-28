## 2025-05-15 - ROM Parsing Missing Edge Cases
**Learning:** Even though the happy path (valid ROMs) was tested, several invalid format variations of ROM files were missing coverage.
**Action:** Always write tests targeting the error variants. When the crate returns an explicit Enum like `RomError`, ensure every variant is triggered at least once.

## 2025-05-15 - Testing TAS Record/Run Coalescing
**Learning:** Run aggregation bugs inside recording modules (like `TasMovie`) can easily manifest if edge cases like adding 0-frames or overflowing the frame counters aren't checked explicitly.
**Action:** When working with types encapsulating arrays of structured runs, push tests targeting the zero-op insertion, bounds checks, and overflow boundaries.

## 2024-05-24 - [MemoryHeatmap Test Coverage]
**Finding:** Uncovered code in `MemoryHeatmap::new` was resolved by adding initialization tests using default parameters and verifying correct sizing of the heap allocations.
**Action:** Always verify `new()` implementations on experimental visualizers and ensure initialization defaults behave exactly as documented. Ensure coverage is maintained on utility/experimental tools as regressions there often signal core breakage later on.
## 2024-05-28 - Test Injection Strategy
**Learning:** Appending `#[cfg(test)] mod tests` to the end of a file that already has a tests module will create duplicate module definitions or fail to scope properly, especially when trying to access private fields or relying on the file's top-level imports inside the new block.
**Action:** When injecting tests autonomously, always read the file to check for an existing `mod tests` block and inject the test functions *inside* that existing block (e.g., using `sed` or targeted `git merge diff` replacement). Additionally, always remember to remove any temporary scratch scripts (like python helpers) used to inject code.
