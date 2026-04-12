## 2025-05-15 - ROM Parsing Missing Edge Cases
**Learning:** Even though the happy path (valid ROMs) was tested, several invalid format variations of ROM files were missing coverage.
**Action:** Always write tests targeting the error variants. When the crate returns an explicit Enum like `RomError`, ensure every variant is triggered at least once.

## 2025-05-15 - Testing TAS Record/Run Coalescing
**Learning:** Run aggregation bugs inside recording modules (like `TasMovie`) can easily manifest if edge cases like adding 0-frames or overflowing the frame counters aren't checked explicitly.
**Action:** When working with types encapsulating arrays of structured runs, push tests targeting the zero-op insertion, bounds checks, and overflow boundaries.
