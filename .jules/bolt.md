## 2025-06-18 - Save State Serialization Optimization
**Learning:** `save_state_file` was taking ownership of `CoreSnapshot` via `.clone()`, doing a 64KB+ deep clone on every save.
**Action:** Changed the `SaveStateFile` struct to use `Cow<'a, CoreSnapshot>` when serializing, completely eliminating the heap allocation when writing the save state file to disk, while keeping deserialization straightforward.
